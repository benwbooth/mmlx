// mmlx — VSCode extension.
//
// CodeLens transport above each `fn name() -> Note`, a spawned `mmlx-server`
// for playback/position, ordinal-based note highlighting, debounced live
// reload, and type-to-play previews. Adapted from lotw's editor extension:
// the protocol here is `load/play/stop/reset/loop/preview` with
// `pos <tick> <ordinal>` reports (one voice, not four channels).

const vscode = require("vscode");
const { Parser, Language, Query } = require("web-tree-sitter");
const cp = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const QUERY = `
(function_item
  name: (identifier) @name
  return_type: (type_identifier) @ret
  body: (block) @body
  (#eq? @ret "Note"))
`;

let ts = null; // Promise<{ parser, query }>
let server = null; // { proc }
let out = null;
let playing = null; // { doc, name, paused }
const loopState = new Map(); // "uri#name" -> bool (default on)
function loopOf(doc, name) {
  const k = `${doc.uri}#${name}`;
  return loopState.has(k) ? loopState.get(k) : true;
}
let debounce = null;
let previewDebounce = null;
let previewHlTimer = null;
let lensChanged = new vscode.EventEmitter();
const cache = new Map(); // doc uri -> { version, fns }
let rollPanel = null;
let rollRows = [];

const highlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: "rgba(120,220,90,0.40)",
  border: "1px solid rgba(120,220,90,0.95)",
  borderRadius: "2px",
  overviewRulerColor: "rgba(120,220,90,1.0)",
  overviewRulerLane: vscode.OverviewRulerLane.Full,
});

function initTreeSitter(ctx) {
  ts = (async () => {
    await Parser.init();
    const parser = new Parser();
    const rust = await Language.load(path.join(ctx.extensionPath, "node_modules/tree-sitter-wasms/out/tree-sitter-rust.wasm"));
    parser.setLanguage(rust);
    return { parser, query: new Query(rust, QUERY) };
  })();
}

async function structure(doc) {
  const key = doc.uri.toString();
  const hit = cache.get(key);
  if (hit && hit.version === doc.version) return hit.val;

  const { parser, query } = await ts;
  const tree = parser.parse(doc.getText());
  const fns = [];
  for (const m of query.matches(tree.rootNode)) {
    const cap = {};
    for (const c of m.captures) cap[c.name] = c.node;
    if (cap.name && cap.body) {
      fns.push({ name: cap.name.text, nameAt: cap.name.startIndex, bodyA: cap.body.startIndex, bodyB: cap.body.endIndex, sections: [] });
    }
  }
  fns.sort((a, b) => a.nameAt - b.nameAt);
  // Sections: top-level ser!/par!/parmin!/fork*! invocations per function
  // (not nested in another one). Played by submitting their source text.
  const SECTIONS = new Set(["ser", "par", "parmin", "forkseq", "forkser", "forkpar"]);
  for (const fn of fns) {
    const fnNode = tree.rootNode.descendantsOfType("function_item").find((f) => {
      const n = f.childForFieldName("name");
      return n && n.startIndex === fn.nameAt;
    });
    if (!fnNode) continue;
    const invocs = fnNode.descendantsOfType("macro_invocation").filter((node) => {
      const macro = node.childForFieldName("macro");
      if (!macro || !SECTIONS.has(macro.text)) return false;
      // top-level: no ancestor macro_invocation of the same family inside this fn
      let parent = node.parent;
      while (parent && parent !== fnNode) {
        if (parent.type === "macro_invocation") {
          const pm = parent.childForFieldName("macro");
          if (pm && SECTIONS.has(pm.text)) return false;
        }
        parent = parent.parent;
      }
      return true;
    });
    invocs.sort((a, b) => a.startIndex - b.startIndex);
    for (const node of invocs) fn.sections.push({ start: node.startIndex, end: node.endIndex });
  }
  cache.set(key, { version: doc.version, val: fns });
  return fns;
}

// Complete sounding tokens: pitched `c4e`/`fs5hdd`/`bb_1t`, rests `rq`.
// (Implicit `c4`, bare `p`, ties need surrounding context — not previewable.)
const VAL = "(?:w|h|q|e|i|t|x)(?:ddd|dd|d)?";
const NOTE_RE = new RegExp(`^(?:[a-g](?:ss|ff|[sfn]|nn)?(?:_\\d|\\d)${VAL}|r${VAL})$`);
// Pitched notes only (no rests): the server counts NoteOns, and rests emit
// none, so highlight ordinals index pitched atoms.
const PITCH_RE = new RegExp(`^[a-g](?:ss|ff|[sfn]|nn)?(?:_\\d|\\d)${VAL}$`);

// Atom-like identifiers in source order within the playing function. The
// server's ordinal (count of NoteOns so far, 1-based) indexes this list.
// Approximate: rests emit no NoteOn, so mapping drifts after rests.
async function songElements(doc, name) {
  const { parser } = await ts;
  const tree = parser.parse(doc.getText());
  const fn = tree.rootNode.descendantsOfType("function_item").find((f) => {
    const n = f.childForFieldName("name");
    return n && n.text === name;
  });
  if (!fn) return [];
  const body = fn.childForFieldName("body");
  const els = [];
  const seen = new Set();
  for (const id of body.descendantsOfType("identifier")) {
    if (NOTE_RE.test(id.text) && !seen.has(id.startIndex)) {
      seen.add(id.startIndex);
      els.push({ a: id.startIndex, b: id.endIndex });
    }
  }
  els.sort((x, y) => x.a - y.a);
  return els;
}

async function previewAtCursor(doc) {
  const ed = vscode.window.visibleTextEditors.find((e) => e.document.uri.toString() === doc.uri.toString());
  if (!ed) return;
  const pos = ed.selection.active;
  const m = doc.lineAt(pos.line).text.slice(0, pos.character).match(/[a-z0-9_]+$/);
  if (!m || !NOTE_RE.test(m[0])) return;
  ensureServer(doc);
  send(`preview ${m[0]}`);
  const idle = () => !playing || playing.paused;
  if (idle()) {
    const range = new vscode.Range(pos.line, pos.character - m[0].length, pos.line, pos.character);
    ed.setDecorations(highlight, [range]);
    clearTimeout(previewHlTimer);
    previewHlTimer = setTimeout(() => { if (idle()) ed.setDecorations(highlight, []); }, 400);
  }
}

function workspaceDir(doc) {
  const f = vscode.workspace.getWorkspaceFolder(doc.uri);
  return f ? f.uri.fsPath : path.dirname(doc.uri.fsPath);
}

function ensureServer(doc) {
  if (server) return server;
  const cfg = vscode.workspace.getConfiguration("mmlx");
  const args = ["run", "--quiet", "-p", "mmlx-server"];
  if (cfg.get("audio", false)) args.push("--features", "audio");
  const proc = cp.spawn("cargo", args, { cwd: workspaceDir(doc), env: { ...process.env, NIX_LDFLAGS: "" } });
  proc.stdout.setEncoding("utf8");
  let buf = "";
  proc.stdout.on("data", (d) => {
    buf += d;
    let nl;
    while ((nl = buf.indexOf("\n")) >= 0) {
      handleEvent(buf.slice(0, nl).trim());
      buf = buf.slice(nl + 1);
    }
  });
  proc.stderr.on("data", (d) => out.append(String(d)));
  proc.on("exit", (c) => { out.appendLine(`server exited (${c})`); server = null; });
  server = { proc };
  return server;
}

function send(cmd) {
  if (server && server.proc.stdin.writable) server.proc.stdin.write(cmd + "\n");
}

// Cold evcxr compiles take minutes with no server output; keep a visible
// "working" message until the server answers with anything.
let busyMsg = null;
function markBusy(text) {
  if (busyMsg) busyMsg.dispose();
  busyMsg = vscode.window.setStatusBarMessage(`$(sync~spin) mmlx: ${text}`);
}
function clearBusy() {
  if (busyMsg) { busyMsg.dispose(); busyMsg = null; }
}

function handleEvent(line) {
  if (!line) return;
  clearBusy();
  if (line.startsWith("pos ")) {
    // `pos <tick> <ordinal> [<inst> <ch> <lane-ordinal>]`
    const parts = line.split(/\s+/);
    const ordinal = Number(parts[2]);
    const lane = parts.length >= 6 && parts[5] !== "-" && Number(parts[5]) >= 1
      ? { inst: parts[3], ch: Number(parts[4]), ord: Number(parts[5]) }
      : null;
    applyHighlight(ordinal, lane);
    if (rollPanel) rollPanel.webview.postMessage({ ordinal });
    return;
  }
  if (line.startsWith("rollrow ")) {
    rollRows.push(line.slice("rollrow ".length));
    return;
  }
  if (line === "rollend") {
    renderRoll();
    return;
  }
  if (line === "ended") {
    if (playing) playing.paused = true;
    const ed = playing && editorFor(playing.doc);
    if (ed) ed.setDecorations(highlight, []);
    lensChanged.fire();
    return;
  }
  out.appendLine(line);
  if (line.startsWith("err ")) {
    vscode.window.setStatusBarMessage("$(warning) mmlx: " + line.slice(4), 5000);
  }
}

function editorFor(doc) {
  const uri = doc.uri.toString();
  return vscode.window.visibleTextEditors.find((e) => e.document.uri.toString() === uri);
}

async function applyHighlight(ordinal, lane) {
  if (!playing || ordinal < 1) return;
  const ed = editorFor(playing.doc);
  if (!ed) return;
  let el = null;
  if (lane && playing.section == null) {
    // Exact lane path: the server pins ym_channel/sn_channel per lane,
    // so the lane ordinal indexes pitched notes with repeat!-expansion.
    const lanes = await laneMap(playing.doc, playing.name);
    const match = lanes.find((l) => l.inst === lane.inst && l.ch === lane.ch);
    if (match) el = match.els[lane.ord - 1] || null;
  }
  if (!el) {
    // Legacy path: global ordinal into all atom-like elements.
    let els;
    if (playing.section != null) {
      // Section play: map the ordinal into the section's own source spans.
      els = await sectionElements(playing.doc, playing.sectionStart, playing.sectionSrc);
    } else {
      els = await songElements(playing.doc, playing.name);
    }
    el = els[ordinal - 1];
  }
  if (!el) return;
  ed.setDecorations(highlight, [new vscode.Range(playing.doc.positionAt(el.a), playing.doc.positionAt(el.b))]);
}

// Voice lanes of a function: the direct ser!/par! children of its outer
// mix par!, in source order, with the instrument + pinned channel parsed
// from each lane's setup and pitched elements with repeat!-expansion.
// The server sends matching (inst, ch, lane-ordinal) in `pos` lines.
const laneCache = new Map(); // uri -> { version, name, lanes }
async function laneMap(doc, name) {
  const key = doc.uri.toString();
  const hit = laneCache.get(key);
  if (hit && hit.version === doc.version && hit.name === name) return hit.lanes;
  const lanes = [];
  const { parser } = await ts;
  const tree = parser.parse(doc.getText());
  const fn = tree.rootNode.descendantsOfType("function_item").find((f) => {
    const n = f.childForFieldName("name");
    return n && n.text === name;
  });
  if (fn) {
    // Outer mix: first top-level par!/parmin! invocation in the function.
    const outer = fn.descendantsOfType("macro_invocation").find((node) => {
      const macro = node.childForFieldName("macro");
      if (!macro || (macro.text !== "par" && macro.text !== "parmin")) return false;
      let parent = node.parent;
      while (parent && parent !== fn) {
        if (parent.type === "macro_invocation") return false;
        parent = parent.parent;
      }
      return true;
    });
    if (outer) {
      const kids = outer.descendantsOfType("macro_invocation").filter((node) => {
        if (node === outer) return false;
        const macro = node.childForFieldName("macro");
        if (!macro || !["ser", "par", "parmin"].includes(macro.text)) return false;
        // Direct child: no other ser/par/parmin/fork macro between.
        let parent = node.parent;
        while (parent && parent !== outer) {
          if (parent.type === "macro_invocation") {
            const pm = parent.childForFieldName("macro");
            if (pm && ["ser", "par", "parmin", "forkseq", "forkser", "forkpar"].includes(pm.text)) return false;
          }
          parent = parent.parent;
        }
        return true;
      });
      kids.sort((a, b) => a.startIndex - b.startIndex);
      const text = doc.getText();
      for (const kid of kids) {
        const src = text.slice(kid.startIndex, kid.endIndex);
        const inst = (src.match(/instrument\s*=\s*"(\w+)"/) || [])[1];
        if (!inst) continue;
        const chm = src.match(/(?:ym_channel|sn_channel)\s*=\s*([\d.]+)/);
        const els = [];
        const seen = new Set();
        let lastPitched = null;
        for (const id of kid.descendantsOfType("identifier")) {
          if (seen.has(id.startIndex)) continue;
          seen.add(id.startIndex);
          if (PITCH_RE.test(id.text)) {
            const el = { a: id.startIndex, b: id.endIndex };
            els.push(el);
            lastPitched = el;
          } else if (id.text === "repeat") {
            const count = (text.slice(id.endIndex).match(/^\((\d+)\)/) || [])[1];
            const extra = count ? Number(count) : 0;
            // repeat!(N) replays the previous atom N more times; rests
            // emit no NoteOns, so only pitched predecessors expand.
            for (let k = 0; k < extra && lastPitched; k++) els.push(lastPitched);
            lastPitched = null; // a repeat is not itself repeatable content
          } else if (!/^(?:param|ser|par|parmin|comment|instrument|tempo|velocity)$/.test(id.text)) {
            lastPitched = null; // anything else breaks a repeat chain
          }
        }
        lanes.push({ inst, ch: chm ? Math.round(Number(chm[1])) : -1, els });
      }
    }
  }
  laneCache.set(key, { version: doc.version, name, lanes });
  return lanes;
}

// Atom-like identifiers in a section's source text, rebased to absolute offsets.
async function sectionElements(doc, base, src) {
  const { parser } = await ts;
  const tree = parser.parse(src);
  const els = [];
  const seen = new Set();
  for (const id of tree.rootNode.descendantsOfType("identifier")) {
    if (NOTE_RE.test(id.text) && !seen.has(id.startIndex)) {
      seen.add(id.startIndex);
      els.push({ a: base + id.startIndex, b: base + id.endIndex });
    }
  }
  els.sort((x, y) => x.a - y.a);
  return els;
}

function isCurrent(doc, name, section) {
  return (
    playing &&
    playing.name === name &&
    playing.doc.uri.toString() === doc.uri.toString() &&
    (section == null ? playing.section == null : playing.section === section)
  );
}

async function playToggle(doc, name, section) {
  if (isCurrent(doc, name, section)) {
    playing.paused = !playing.paused;
    if (playing.paused) {
      // Pause freezes both clocks and drops queued audio.
      playing.frozenText = doc.getText();
      send("stop");
    } else if (playing.frozenText !== undefined && playing.frozenText !== doc.getText()) {
      // Edited while paused: re-evaluate, keeping position.
      playing.frozenText = undefined;
      const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
      fs.writeFileSync(tmp, doc.getText());
      send(`load ${tmp}`);
      send(`reload`);
      markBusy("reloading…");
    } else {
      // Untouched: continue without re-evaluating (instant).
      playing.frozenText = undefined;
      send("resume");
    }
    lensChanged.fire();
  } else if (section != null) {
    await playSection(doc, name, section);
  } else {
    await play(doc, name);
  }
}

async function play(doc, name) {
  ensureServer(doc);
  out.appendLine(`▶ ${name}`);
  playing = { doc, name, section: null, paused: false };
  send(`loop ${loopOf(doc, name) ? "on" : "off"}`);
  if (bufferMatchesDisk(doc)) {
    // Saved file: the server plays its compiled-in copy instantly
    // (lotw `rom` path), no load/compile wait.
    send(`play ${name}()`);
  } else {
    // Unsaved edits: load the buffer through the JIT, then play.
    writeAndSend();
  }
  markBusy("loading…");
  lensChanged.fire();
}

// True when the editor buffer is byte-identical to the file on disk
// (or the file can't be read, e.g. untitled — then false → JIT path).
function bufferMatchesDisk(doc) {
  try {
    return fs.readFileSync(doc.uri.fsPath, "utf8") === doc.getText();
  } catch {
    return false;
  }
}

// Sections play by submitting their (whitespace-collapsed) source text.
// Only self-contained sections work: references to fn locals fail to eval
// (the server error shows in the status bar).
async function playSection(doc, name, index) {
  ensureServer(doc);
  const fns = await structure(doc);
  const fn = fns.find((f) => f.name === name);
  const section = fn && fn.sections[index];
  if (!section) return;
  const src = doc.getText(new vscode.Range(doc.positionAt(section.start), doc.positionAt(section.end))).replace(/\s+/g, " ");
  out.appendLine(`▶ ${name} §${index + 1}`);
  playing = { doc, name, section: index, sectionSrc: src, sectionStart: section.start, paused: false };
  send(`loop ${loopOf(doc, name) ? "on" : "off"}`);
  const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
  fs.writeFileSync(tmp, doc.getText());
  send(`load ${tmp}`);
  send(`play ${src}`);
  markBusy("loading…");
  lensChanged.fire();
}

function writeAndSend() {
  if (!playing) return;
  const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
  fs.writeFileSync(tmp, playing.doc.getText());
  send(`load ${tmp}`);
  send(`play ${playing.name}()`);
}

function rollHtml(rows) {
  const trs = rows.map((r, i) => {
    const [start, midi, dur, inst] = r.split(/\s+/);
    return `<tr id="row${i + 1}"><td>${i + 1}</td><td>${start}</td><td>${midi}</td><td>${dur}</td><td>${inst || ""}</td></tr>`;
  }).join("");
  return `<!DOCTYPE html><html><body>
<style>table{border-collapse:collapse;font-family:monospace}td,th{border:1px solid #555;padding:2px 8px}.on{background:rgba(120,220,90,.45)}</style>
<table><tr><th>#</th><th>start</th><th>midi</th><th>dur</th><th>inst</th></tr>${trs}</table>
<script>const vscode=acquireVsCodeApi();window.addEventListener('message',e=>{document.querySelectorAll('.on').forEach(el=>el.classList.remove('on'));const el=document.getElementById('row'+e.data.ordinal);if(el)el.classList.add('on');});</script>
</body></html>`;
}

function renderRoll() {
  if (!rollPanel) return;
  rollPanel.webview.html = rollHtml(rollRows);
}

async function showRoll(doc, name) {
  ensureServer(doc);
  if (!rollPanel) {
    rollPanel = vscode.window.createWebviewPanel("mmlxRoll", "mmlx piano roll", vscode.ViewColumn.Beside, { enableScripts: true });
    rollPanel.onDidDispose(() => { rollPanel = null; rollRows = []; });
  }
  rollRows = [];
  const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
  fs.writeFileSync(tmp, doc.getText());
  send(`load ${tmp}`);
  send(`roll ${name}()`);
  markBusy("loading…");
}

async function reloadIfPlaying() {
  if (!playing || playing.paused) return;
  const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
  fs.writeFileSync(tmp, playing.doc.getText());
  send(`load ${tmp}`);
  send(`reload`); // server re-evaluates the current expr, keeping position
  markBusy("reloading…");
}

async function functionAt(doc, line) {
  const fns = await structure(doc);
  let best = null;
  for (const f of fns) if (doc.positionAt(f.nameAt).line <= line) best = f;
  return best;
}

class Lenses {
  get onDidChangeCodeLenses() { return lensChanged.event; }
  async provideCodeLenses(doc) {
    const fns = await structure(doc);
    const lenses = [];
    const at = (off) => { const p = doc.positionAt(off); return new vscode.Range(p, p); };
    for (const fn of fns) {
      const r = at(fn.nameAt);
      const cur = isCurrent(doc, fn.name, null);
      lenses.push(new vscode.CodeLens(r, { title: cur && !playing.paused ? "⏸ Pause" : "▶ Play", command: "mmlx.play", arguments: [doc, fn.name] }));
      lenses.push(new vscode.CodeLens(r, { title: "⏹ Stop", command: "mmlx.stop" }));
      const lon = loopOf(doc, fn.name);
      lenses.push(new vscode.CodeLens(r, { title: `🔁 Loop ${lon ? "on" : "off"}`, command: "mmlx.toggleLoop", arguments: [doc, fn.name] }));
      fn.sections.forEach((section, k) => {
        const sr = at(section.start);
        const scur = isCurrent(doc, fn.name, k);
        lenses.push(new vscode.CodeLens(sr, { title: scur && !playing.paused ? `⏸ §${k + 1}` : `▶ §${k + 1}`, command: "mmlx.playSection", arguments: [doc, fn.name, k] }));
      });
    }
    return lenses;
  }
}

function activate(ctx) {
  out = vscode.window.createOutputChannel("mmlx");
  out.appendLine("mmlx activated");
  initTreeSitter(ctx);
  ctx.subscriptions.push(vscode.languages.registerCodeLensProvider({ language: "rust" }, new Lenses()));

  ctx.subscriptions.push(
    vscode.commands.registerCommand("mmlx.play", async (doc, name) => {
      if (!doc) {
        const ed = vscode.window.activeTextEditor;
        if (!ed) return;
        const fn = await functionAt(ed.document, ed.selection.active.line);
        if (!fn) return vscode.window.showInformationMessage("No fn() -> Note under the cursor.");
        doc = ed.document;
        name = fn.name;
      }
      playToggle(doc, name);
    }),
    vscode.commands.registerCommand("mmlx.playSection", (doc, name, section) => playToggle(doc, name, section)),
    vscode.commands.registerCommand("mmlx.stop", () => {
      send("reset");
      const ed = playing ? editorFor(playing.doc) : vscode.window.activeTextEditor;
      if (ed) ed.setDecorations(highlight, []);
      playing = null;
      lensChanged.fire();
    }),
    vscode.commands.registerCommand("mmlx.toggleLoop", (doc, name) => {
      const on = !loopOf(doc, name);
      loopState.set(`${doc.uri}#${name}`, on);
      if (playing && playing.name === name) send(`loop ${on ? "on" : "off"}`);
      lensChanged.fire();
    }),
    vscode.commands.registerCommand("mmlx.showRoll", async () => {
      const ed = vscode.window.activeTextEditor;
      if (!ed) return;
      const fn = await functionAt(ed.document, ed.selection.active.line);
      if (!fn) return vscode.window.showInformationMessage("No fn() -> Note under the cursor.");
      showRoll(ed.document, fn.name);
    })
  );

  ctx.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document.languageId !== "rust") return;
      lensChanged.fire();
      clearTimeout(previewDebounce);
      previewDebounce = setTimeout(() => previewAtCursor(e.document), 120);
      if (!playing || e.document !== playing.doc) return;
      clearTimeout(debounce);
      const ms = vscode.workspace.getConfiguration("mmlx").get("debounceMs", 300);
      debounce = setTimeout(reloadIfPlaying, ms);
    })
  );
}

function deactivate() {
  if (server) server.proc.kill();
}

module.exports = { activate, deactivate };
