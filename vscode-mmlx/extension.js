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
      fns.push({ name: cap.name.text, nameAt: cap.name.startIndex, bodyA: cap.body.startIndex, bodyB: cap.body.endIndex });
    }
  }
  fns.sort((a, b) => a.nameAt - b.nameAt);
  cache.set(key, { version: doc.version, val: fns });
  return fns;
}

// Complete sounding tokens: pitched `c4e`/`fs5hdd`/`bb_1t`, rests `rq`.
// (Implicit `c4`, bare `p`, ties need surrounding context — not previewable.)
const VAL = "(?:w|h|q|e|i|t|x)(?:ddd|dd|d)?";
const NOTE_RE = new RegExp(`^(?:[a-g](?:ss|ff|[sfn]|nn)?(?:_\\d|\\d)${VAL}|r${VAL})$`);

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

function handleEvent(line) {
  if (!line) return;
  if (line.startsWith("pos ")) {
    const ordinal = Number(line.split(/\s+/)[2]);
    applyHighlight(ordinal);
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

async function applyHighlight(ordinal) {
  if (!playing || ordinal < 1) return;
  const ed = editorFor(playing.doc);
  if (!ed) return;
  const els = await songElements(playing.doc, playing.name);
  const el = els[ordinal - 1];
  if (!el) return;
  ed.setDecorations(highlight, [new vscode.Range(playing.doc.positionAt(el.a), playing.doc.positionAt(el.b))]);
}

function isCurrent(doc, name) {
  return playing && playing.name === name && playing.doc.uri.toString() === doc.uri.toString();
}

async function playToggle(doc, name) {
  if (isCurrent(doc, name)) {
    playing.paused = !playing.paused;
    send(playing.paused ? "stop" : `play ${name}()`);
    if (!playing.paused) {
      // resume restarts the stream from the top (v1: no position memory)
    }
    lensChanged.fire();
  } else {
    await play(doc, name);
  }
}

async function play(doc, name) {
  ensureServer(doc);
  out.appendLine(`▶ ${name}`);
  playing = { doc, name, paused: false };
  send(`loop ${loopOf(doc, name) ? "on" : "off"}`);
  writeAndSend();
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
}

async function reloadIfPlaying() {
  if (!playing || playing.paused) return;
  writeAndSend(); // v1: reload restarts from the top
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
      const cur = isCurrent(doc, fn.name);
      lenses.push(new vscode.CodeLens(r, { title: cur && !playing.paused ? "⏸ Pause" : "▶ Play", command: "mmlx.play", arguments: [doc, fn.name] }));
      lenses.push(new vscode.CodeLens(r, { title: "⏹ Stop", command: "mmlx.stop" }));
      const lon = loopOf(doc, fn.name);
      lenses.push(new vscode.CodeLens(r, { title: `🔁 Loop ${lon ? "on" : "off"}`, command: "mmlx.toggleLoop", arguments: [doc, fn.name] }));
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
