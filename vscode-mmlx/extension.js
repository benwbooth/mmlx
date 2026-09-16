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
  return_type: (_) @ret
  body: (block) @body)
`;
// Return types counting as songs: plain notes plus generator songs
// (`impl IntoIterator<Item = Note> ...` has no single identifier).
const SONG_RET = /Note|SongStream/;

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
  backgroundColor: "rgba(255,235,59,0.35)",
  border: "1px solid rgba(255,235,59,0.95)",
  borderRadius: "2px",
  overviewRulerColor: "rgba(255,235,59,1.0)",
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
    if (cap.name && cap.body && cap.ret && SONG_RET.test(cap.ret.text)) {
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
        // Voice `let` initializers (`let voice: Note = ser!(...)`) are
        // bindings, not playable sections.
        if (parent.type === "let_declaration") return false;
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
// VAL covers all durations incl. `o` (the shortest); bare ties stay out.
const VAL = "(?:w|h|q|e|i|t|x|o)(?:ddd|dd|d)?";
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
    // `pos <tick> <ordinal> [<inst> <ch> <lane-ordinal> [<cycle>]]`
    const parts = line.split(/\s+/);
    const ordinal = Number(parts[2]);
    const lane = parts.length >= 6 && parts[5] !== "-" && Number(parts[5]) >= 1
      ? { inst: parts[3], ch: Number(parts[4]), ord: Number(parts[5]) }
      : null;
    // Generator-song body index (0 = intro): picks the lane section.
    const cycle = parts.length >= 7 ? Number(parts[6]) || 0 : 0;
    if (playing) playing.cycle = cycle;
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
    // Generator songs restart ordinals per body; the cycle picks intro
    // (first body) vs loop.
    const sections = await laneMap(playing.doc, playing.name);
    const lanes = (playing.cycle >= 1 && sections.loop.length > 0) ? sections.loop : sections.intro;
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
  // Follow window: pos events carry NoteOns only, so recency approximates
  // the sounding set. Highlight them all and keep the screen on the music:
  // center the oldest, expanding to fit everything sounding if it fits.
  const now = Date.now();
  playing.recent = (playing.recent || []).filter((e) => now - e.t < FOLLOW_MS);
  playing.recent.push({ a: el.a, b: el.b, t: now });
  if (playing.recent.length > 60) playing.recent.splice(0, playing.recent.length - 60);
  ed.setDecorations(
    highlight,
    playing.recent.map((e) => new vscode.Range(playing.doc.positionAt(e.a), playing.doc.positionAt(e.b)))
  );
  followRecent(ed);
}

// Sounding-note window (ms) for follow-playback.
const FOLLOW_MS = 2000;

// Pure follow decision, headless-testable. `spans` are sounding notes in
// time order (oldest first) as line spans; the viewport is inclusive.
// Returns 'full' (min..max fits: show it all, centered), 'first' (too
// tall: center the oldest), or 'none' (oldest already visible).
function pickReveal(spans, visStart, visEnd) {
  if (!spans.length) return { action: "none" };
  const first = spans[0];
  if (first.aLine >= visStart && first.aLine <= visEnd) return { action: "none" };
  let lo = Infinity;
  let hi = -Infinity;
  for (const s of spans) {
    lo = Math.min(lo, s.aLine);
    hi = Math.max(hi, s.bLine);
  }
  const visLines = Math.max(1, visEnd - visStart + 1);
  if (hi - lo + 1 <= visLines) return { action: "full", lo, hi };
  return { action: "first", lo: first.aLine, hi: first.bLine };
}

function followRecent(ed) {
  const recent = playing && playing.recent;
  if (!recent || !recent.length) return;
  const doc = playing.doc;
  const spans = recent.map((e) => {
    const a = doc.positionAt(e.a);
    const b = doc.positionAt(e.b);
    return { a: e.a, b: e.b, aLine: a.line, bLine: b.line };
  });
  const vis = ed.visibleRanges && ed.visibleRanges[0];
  const pick = vis
    ? pickReveal(spans, vis.start.line, vis.end.line)
    : pickReveal(spans, -1, -2);
  if (pick.action === "none") return;
  let minA = Infinity;
  let maxB = -Infinity;
  for (const e of recent) {
    minA = Math.min(minA, e.a);
    maxB = Math.max(maxB, e.b);
  }
  const range =
    pick.action === "full"
      ? new vscode.Range(doc.positionAt(minA), doc.positionAt(maxB))
      : new vscode.Range(doc.positionAt(spans[0].a), doc.positionAt(spans[0].b));
  ed.revealRange(range, vscode.TextEditorRevealType.InCenter);
}

// Voice lanes of a function, bar-major: the outer mix is a top-level
// `ser!` of `par!` bars, each a score-ordered stack of channel `ser!`s.
// Lanes collect per-channel pitched elements across bars in time order,
// with the instrument + pinned channel parsed per channel (direct pins,
// bare `voice_*` lets, staff comments as fallback) and seg-phrase
// expansion like before. The server sends matching (inst, ch,
// lane-ordinal, cycle) in `pos` lines; generator songs restart ordinals
// per yielded body, picked by cycle (0 = intro).
//
// NOTE: tree-sitter-rust keeps macro bodies as opaque token soup, so the
// mix/bar/channel splits below scan brackets in text instead of walking
// macro_invocation AST nodes.
const laneCache = new Map(); // uri -> { version, name, lanes }
// Strip leading line/block comments (staff labels like `// melody (psg)`)
// so lane-head detection sees the `ser!`/`par!` underneath.
function stripLeadingComments(s) {
  let t = s;
  for (;;) {
    const m = t.match(/^\s*(\/\/[^\n]*\n|\/\*[\s\S]*?\*\/)/);
    if (!m) return t;
    t = t.slice(m[0].length);
  }
}

async function laneMap(doc, name) {
  const key = doc.uri.toString();
  const hit = laneCache.get(key);
  if (hit && hit.version === doc.version && hit.name === name) return hit.lanes;
  const lanes = [];
  const { parser } = await ts;
  const text = doc.getText();
  const tree = parser.parse(text);
  const fn = tree.rootNode.descendantsOfType("function_item").find((f) => {
    const n = f.childForFieldName("name");
    return n && n.text === name;
  });
  if (fn) {
    // Mix roots to walk: a plain song fn has one tail-expression mix;
    // a generator fn (`gen!` block) yields one mix per body (intro first,
    // then loop). Voice lets precede them in both shapes.
    const MIX = new Set(["ser", "par", "parmin"]);
    const FAMILY = new Set(["ser", "par", "parmin", "forkseq", "forkser", "forkpar"]);
    const isTop = (node) => {
      let p = node.parent;
      while (p && p !== fn) {
        if (p.type === "macro_invocation") {
          const pm = p.childForFieldName("macro");
          if (pm && FAMILY.has(pm.text)) return false;
        }
        p = p.parent;
      }
      return true;
    };
    // Top-level `yield_!` argument spans inside a gen! body, in order.
    function splitYields(genSrc, base) {
      const items = [];
      const end = genSrc.length;
      let i = 0;
      let mode = null; // "str", "chr", "line", "block"
      const isIdent = (ch) => /[A-Za-z0-9_]/.test(ch);
      while (i < end) {
        const ch = genSrc[i];
        const nx = i + 1 < end ? genSrc[i + 1] : "";
        if (mode === "str") {
          if (ch === "\\") i++;
          else if (ch === '"') mode = null;
        } else if (mode === "chr") {
          if (ch === "\\") i++;
          else if (ch === "'") mode = null;
        } else if (mode === "line") {
          if (ch === "\n") mode = null;
        } else if (mode === "block") {
          if (ch === "*" && nx === "/") { mode = null; i++; }
        } else if (ch === '"') {
          mode = "str";
        } else if (ch === "'") {
          mode = "chr";
        } else if (ch === "/" && nx === "/") {
          mode = "line";
        } else if (ch === "/" && nx === "*") {
          mode = "block";
        } else if (
          genSrc.startsWith("yield_", i) &&
          !isIdent(genSrc[i - 1] || " ") &&
          /^yield_!\s*\(/.test(genSrc.slice(i))
        ) {
          const open = genSrc.indexOf("(", i);
          let dd = 0;
          let k = open;
          let mm = null;
          // Balanced parens from the yield's open paren.
          for (let j = open; j < end; j++) {
            const c2 = genSrc[j];
            const n2 = j + 1 < end ? genSrc[j + 1] : "";
            if (mm === "str") {
              if (c2 === "\\") j++;
              else if (c2 === '"') mm = null;
            } else if (mm === "chr") {
              if (c2 === "\\") j++;
              else if (c2 === "'") mm = null;
            } else if (mm === "line") {
              if (c2 === "\n") mm = null;
            } else if (mm === "block") {
              if (c2 === "*" && n2 === "/") { mm = null; j++; }
            } else if (c2 === '"') {
              mm = "str";
            } else if (c2 === "'") {
              mm = "chr";
            } else if (c2 === "/" && n2 === "/") {
              mm = "line";
            } else if (c2 === "/" && n2 === "*") {
              mm = "block";
            } else if (c2 === "(" || c2 === "[" || c2 === "{") {
              dd++;
            } else if (c2 === ")" || c2 === "]" || c2 === "}") {
              dd--;
              if (dd === 0) { k = j; break; }
            }
          }
          if (dd !== 0) break; // unbalanced; give up
          const a = open + 1;
          const src = genSrc.slice(a, k).trim();
          if (src) items.push({ a: base + a, b: base + k, src });
          i = k + 1;
          continue;
        }
        i++;
      }
      return items;
    }
    const genNode = fn.descendantsOfType("macro_invocation").find((node) => {
      const macro = node.childForFieldName("macro");
      return macro && macro.text === "gen" && isTop(node);
    });
    // Mix roots: generator yields in order, else the single tail mix.
    let introRoots = [];
    let loopRoots = [];
    if (genNode) {
      const genSrc = text.slice(genNode.startIndex, genNode.endIndex);
      const yields = splitYields(genSrc, genNode.startIndex);
      if (yields.length > 0) {
        introRoots = [yields[0]];
        loopRoots = yields.slice(1);
      }
    } else {
      const outers = fn.descendantsOfType("macro_invocation").filter((node) => {
        const macro = node.childForFieldName("macro");
        return macro && MIX.has(macro.text) && isTop(node);
      });
      const outer = outers[outers.length - 1];
      if (outer) {
        introRoots = [{
          a: outer.startIndex,
          b: outer.endIndex,
          src: text.slice(outer.startIndex, outer.endIndex),
        }];
      }
    }
    const isBarHead = (s) => /^(?:par|parmin|bar)\s*!/.test(stripLeadingComments(s).trim());
    const isChHead = (s) => /^(?:ser|par|parmin|forkseq|forkser|forkpar|track)\s*!/.test(stripLeadingComments(s).trim());
    // Voice programs hoist instrument + channel pins out of lanes
    // (bare `voice_melody` splices a `let voice_melody` binding — blocks
    // borrow items; older songs call `voice_melody()` fns or splice
    // `voice_melody.clone()`), so resolve lane identity through voice
    // definitions when the lane itself carries no pin. Staff comments
    // (`// melody (psg)`) are a final fallback.
    const voiceMap = new Map();
    for (const m of text.matchAll(/fn\s+(voice_\w+)\s*\(\)\s*->\s*Note\s*\{([\s\S]*?)\n\}/g)) {
      const body = m[2];
      const vi = (body.match(/instrument\s*=\s*"(\w+)"/) || [])[1];
      const vc = (body.match(/(?:ym_channel|sn_channel)\s*=\s*([\d.]+)/) || [])[1];
      if (vi) voiceMap.set(m[1], { inst: vi, ch: vc !== undefined ? Math.round(Number(vc)) : -1 });
    }
    for (const m of text.matchAll(/let\s+(voice_\w+)\s*:\s*Note\s*=\s*([^;]+);/g)) {
      if (voiceMap.has(m[1])) continue;
      const body = m[2];
      const vi = (body.match(/instrument\s*=\s*"(\w+)"/) || [])[1];
      const vc = (body.match(/(?:ym_channel|sn_channel)\s*=\s*([\d.]+)/) || [])[1];
      if (vi) voiceMap.set(m[1], { inst: vi, ch: vc !== undefined ? Math.round(Number(vc)) : -1 });
    }
    // Phrase functions (`bassl_seg_3()`) splice their bodies inline at
    // every call site, so highlight ordinals index the *expanded* note
    // stream. Expand seg calls to their definition offsets (shared
    // across call sites, like the phrase itself).
    const PRE = "fn _w() -> Note { ";
    const segBodies = new Map();
    for (const m of text.matchAll(/fn\s+(\w+_seg_\d+)\s*\(\)\s*->\s*Note\s*\{([\s\S]*?)\n\}/g)) {
      const body = m[2];
      segBodies.set(m[1], { src: body, start: m.index + m[0].indexOf(body) });
    }
    const segElsCache = new Map();
    const NONPITCH_OK = /^(?:param|ser|par|parmin|instrument|tempo|velocity|bar|track)$/;
    // Pitched elements (with repeat!/seg expansion) of one source unit,
    // rebased to absolute document offsets. Shared by channels and segs.
    function walkEls(src, absBase, stack) {
      const tree = parser.parse(PRE + src + " }");
      const fnNode = tree.rootNode.descendantsOfType("function_item")[0];
      const ids = fnNode ? fnNode.descendantsOfType("identifier") : [];
      const els = [];
      const seen = new Set();
      let lastPitched = null;
      for (const id of ids) {
        const a = absBase + id.startIndex - PRE.length;
        const b = absBase + id.endIndex - PRE.length;
        if (seen.has(a)) continue;
        seen.add(a);
        if (PITCH_RE.test(id.text)) {
          const el = { a, b };
          els.push(el);
          lastPitched = el;
        } else if (id.text === "repeat") {
          const count = (text.slice(b).match(/^!\((\d+)\)/) || [])[1];
          const extra = count ? Number(count) : 0;
          // repeat!(N) replays the previous atom N more times; rests
          // emit no NoteOns, so only pitched predecessors expand.
          for (let k = 0; k < extra && lastPitched; k++) els.push(lastPitched);
          lastPitched = null; // a repeat is not itself repeatable content
        } else if (segBodies.has(id.text)) {
          // Phrase call: splice the definition's notes inline so ordinals
          // track the expanded stream the server counts.
          els.push(...segPitchEls(id.text, stack));
          lastPitched = null;
        } else if (!NONPITCH_OK.test(id.text)) {
          lastPitched = null; // anything else breaks a repeat chain
        }
      }
      return els;
    }
    function segPitchEls(segName, stack) {
      if (segElsCache.has(segName)) return segElsCache.get(segName);
      if (stack.includes(segName)) return [];
      const body = segBodies.get(segName);
      if (!body) return [];
      // Placeholder breaks reference cycles (segs never nest, but stay safe).
      segElsCache.set(segName, []);
      const els = walkEls(body.src, body.start, stack.concat([segName]));
      segElsCache.set(segName, els);
      return els;
    }
    // Resolve a channel's (inst, ch): direct pins, then voice lets/calls,
    // then staff comments (`// melody (psg)`).
    function channelId(src) {
      const clean = stripLeadingComments(src).trim();
      let inst = (clean.match(/instrument\s*=\s*"(\w+)"/) || [])[1];
      let chm = clean.match(/(?:ym_channel|sn_channel)\s*=\s*([\d.]+)/);
      if (!inst) {
        const vcall = (clean.match(/\b(voice_\w+)\b/) || [])[1];
        const v = vcall && voiceMap.get(vcall);
        if (v) { inst = v.inst; if (!chm && v.ch >= 0) chm = [null, String(v.ch)]; }
      }
      if (!inst) inst = (src.match(/\/\/\s*[\w-]+\s*\((\w+)\)/) || [])[1];
      if (!inst) return null;
      return { inst, ch: chm ? Math.round(Number(chm[1])) : -1 };
    }
    // Walk one mix root (a ser!/par! mix invocation) into per-channel
    // lanes in score order, appending elements across its bars.
    function walkRoot(rootSrc, rootBase, laneByKey, laneOrder) {
      for (const item of splitNested(rootSrc, rootBase)) {
        if (!isBarHead(item.src)) continue;
        for (const ch of splitNested(item.src, item.a)) {
          if (!isChHead(ch.src)) continue;
          const id = channelId(ch.src);
          if (!id) continue;
          const key = id.inst + ":" + id.ch;
          let lane = laneByKey.get(key);
          if (!lane) {
            lane = { inst: id.inst, ch: id.ch, els: [] };
            laneByKey.set(key, lane);
            laneOrder.push(lane);
          }
          lane.els.push(...walkEls(ch.src, ch.a, []));
        }
      }
    }
    // Intro section = first yield (or the whole mix for plain fns);
    // loop section = later yields concatenated in order.
    const introByKey = new Map();
    const introOrder = [];
    for (const root of introRoots) {
      walkRoot(root.src, root.a, introByKey, introOrder);
    }
    const loopByKey = new Map();
    const loopOrder = [];
    for (const root of loopRoots) {
      walkRoot(root.src, root.a, loopByKey, loopOrder);
    }
    // Noteless lanes (all-rest staves, identified by comments alone) can
    // never be highlight targets: ordinals count NoteOns.
    const intro = introOrder.filter((l) => l.els.length > 0);
    const loop = loopOrder.filter((l) => l.els.length > 0);
    laneCache.set(key, { version: doc.version, name, lanes: { intro, loop } });
    return { intro, loop };
  }
  laneCache.set(key, { version: doc.version, name, lanes: { intro: [], loop: [] } });
  return { intro: [], loop: [] };
}

// Split a macro body's top-level items (with absolute offsets), aware
// of nested brackets, strings, char literals, and line/block comments.
// Supports both the bracket form `par!([...])` and the terse paren form
// `par!(...)` (no brackets, space/comma-separated). Returns [] when not
// found.
const MIX_HEAD = /^(?:ser|par|parmin)\s*!\s*\(\s*(\[?)/;
const NEST_HEAD = /^(?:ser|par|parmin|forkseq|forkser|forkpar|bar|track)\s*!\s*\(\s*(\[?)/;
function splitTopLevel(text, fn, outer) {
  if (!outer) return [];
  const head = text.slice(outer.startIndex, outer.endIndex);
  const m = head.match(MIX_HEAD);
  if (!m) return [];
  return scanItems(text, outer.startIndex + m[0].length, outer.endIndex, m[1] === "[");
}

// Split a nested macro invocation given its source text and absolute base
// offset (for bar pars and channel sers inside the opaque mix soup).
function splitNested(src, base) {
  const m = src.match(NEST_HEAD);
  if (!m) return [];
  return scanItems(src, m[0].length, src.length, m[1] === "[").map((it) => ({
    a: it.a + base,
    b: it.b + base,
    src: it.src,
  }));
}

function scanItems(text, start, end, bracket) {
  let depth = 0;
  let closed = false;
  let i = start;
  const items = [];
  let mode = null; // "str", "chr", "line", "block"
  let cur = i;
  const push = (b) => {
    let a = cur;
    while (a < b && /\s/.test(text[a])) a++;
    const src = text.slice(a, b).trim();
    if (src) items.push({ a, b, src });
    cur = b + 1;
  };
  while (i < end) {
    const ch = text[i];
    const nx = i + 1 < end ? text[i + 1] : "";
    if (mode === "str") {
      if (ch === "\\") i++;
      else if (ch === '"') mode = null;
    } else if (mode === "chr") {
      if (ch === "\\") i++;
      else if (ch === "'") mode = null;
    } else if (mode === "line") {
      if (ch === "\n") mode = null;
    } else if (mode === "block") {
      if (ch === "*" && nx === "/") { mode = null; i++; }
    } else if (ch === '"') {
      mode = "str";
    } else if (ch === "'") {
      mode = "chr";
    } else if (ch === "/" && nx === "/") {
      mode = "line";
    } else if (ch === "/" && nx === "*") {
      mode = "block";
    } else if (ch === "[" || ch === "(" || ch === "{") {
      depth++;
    } else if (ch === "]" || ch === ")" || ch === "}") {
      if (depth === 0) {
        if ((bracket && ch === "]") || (!bracket && ch === ")")) push(i);
        closed = true;
        break;
      }
      depth--;
    } else if (ch === "," && depth === 0) {
      push(i);
    }
    i++;
  }
  if (!closed && cur < end) push(end); // trailing item when the closer is outside (nested src)
  return items;
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
  playing = { doc, name, section: null, paused: false, recent: [] };
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
// Voice `let` bindings live outside sections, so the fn's single-line
// lets ride along; otherwise references like bare `voice_melody` splices
// fail to eval (the server error shows in the status bar).
async function playSection(doc, name, index) {
  ensureServer(doc);
  const fns = await structure(doc);
  const fn = fns.find((f) => f.name === name);
  const section = fn && fn.sections[index];
  if (!section) return;
  const bodyText = doc.getText(new vscode.Range(doc.positionAt(fn.bodyA), doc.positionAt(fn.bodyB)));
  const lets = [...bodyText.matchAll(/^[ \t]*let\s+\w+\s*:[^;\n]+;[ \t]*$/gm)]
    .map((m) => m[0].trim().replace(/\s+/g, " "))
    .join(" ");
  const src = doc.getText(new vscode.Range(doc.positionAt(section.start), doc.positionAt(section.end))).replace(/\s+/g, " ");
  out.appendLine(`▶ ${name} §${index + 1}`);
  playing = { doc, name, section: index, sectionSrc: src, sectionStart: section.start, paused: false, recent: [] };
  send(`loop ${loopOf(doc, name) ? "on" : "off"}`);
  const tmp = path.join(os.tmpdir(), `mmlx_${process.pid}.rs`);
  fs.writeFileSync(tmp, doc.getText());
  send(`load ${tmp}`);
  send(`play { ${lets} ${src} }`);
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
    // Never throw: a failed lens pass must not blank the transport.
    let fns = [];
    try {
      fns = await structure(doc);
    } catch (e) {
      out.appendLine(`lenses: structure failed (${e && e.message || e})`);
      return [];
    }
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
// Exported for headless testing (node harness with a vscode stub).
module.exports.__test = { laneMap, songElements, sectionElements, structure, pickReveal, NOTE_RE, PITCH_RE, initTreeSitter, splitTopLevel, splitNested };
