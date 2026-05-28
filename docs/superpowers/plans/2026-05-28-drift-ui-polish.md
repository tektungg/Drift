# Drift UI/UX Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the UI/UX polish from `docs/superpowers/specs/2026-05-28-drift-ui-polish-design.md` — line icons, state-colored rows, refined sidebar, empty state, sectioned settings with toggle switches — then bump the version to 0.2.0.

**Architecture:** Frontend-only. All changes live in `src/styles.css`, `src/main.js`, and a new `src/icons.js`. The Rust backend is untouched (it already emits everything the UI needs). Version bump touches `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.

**Tech Stack:** Vanilla HTML/CSS/JS in a Tauri 2 webview. No build step for the frontend (files are served as-is from `src/`). Verify by `cargo tauri dev` (visual) or `cargo build` in `src-tauri/` (proves nothing broke; the frontend isn't compiled).

---

## Conventions for the implementer

- Working dir: `D:\Personal Project\Drift\`. Windows + PowerShell. Use the PowerShell tool for cargo; Bash tool for git.
- Cargo PATH refresh before any cargo command:
  ```powershell
  $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
  ```
- There are **no automated frontend tests**. "Verification" = (a) `node -e "require('fs').readFileSync('<file>','utf8')"` to confirm the JS file parses as text (sanity), and (b) `cargo build` in `src-tauri/` stays clean (proves the Rust/Tauri side is unaffected). Real visual QA happens in the smoke checklist (Task 7).
- The existing `src/main.js` uses these functions you will modify: `renderSidebar`, `renderList`, `rowHtml`, `renderExpanded`, `toggleSettings`, `openAddDialog`'s `renderMeta`. Existing palette tokens are in `:root` in `src/styles.css`.
- Commit after every task with the message shown in its final step.
- Do NOT touch any `.rs` file except where a task explicitly says so (only Task 7 touches non-frontend files, and only Cargo.toml/tauri.conf.json for the version).

---

## Task 1: Design tokens + icon module

**Files:**
- Modify: `src/styles.css` (add tokens + state-color + icon-square + switch classes)
- Create: `src/icons.js`

- [ ] **Step 1: Add tokens and shared classes to `src/styles.css`**

Find the `:root { … }` block and add these variables before its closing `}`:

```css
  --st-downloading: #D97757;
  --st-seeding:     #7D9B76;
  --st-completed:   #6E8F86;
  --st-paused:      #A8A096;
  --st-stalled:     #CDA04E;
  --st-error:       #C0573C;
  --prog-track:     #ECE5D6;
  --icon-bg:        #EFE9DB;
  --icon-fg:        #6B645A;
```

Then append this block at the END of `src/styles.css`:

```css
/* ── UI polish: state colors, icons, switches ── */
.st-downloading { color: var(--st-downloading); }
.st-seeding     { color: var(--st-seeding); }
.st-completed   { color: var(--st-completed); }
.st-paused      { color: var(--st-paused); }
.st-stalled     { color: var(--st-stalled); }
.st-error       { color: var(--st-error); }
.st-initializing{ color: var(--ink-soft); }

.state-dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block;
  margin-right: 6px; background: currentColor; }

.ficon { width: 30px; height: 30px; border-radius: 7px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  background: var(--icon-bg); color: var(--icon-fg); }
.ficon svg { width: 16px; height: 16px; }
.ficon.video { background: var(--accent-soft); }
.ficon.audio { background: #E3EAD9; }

/* Toggle switch (settings booleans) */
.switch { width: 34px; height: 20px; border-radius: 99px; background: var(--accent);
  position: relative; flex-shrink: 0; cursor: pointer; border: 0; padding: 0;
  transition: background 0.18s ease; }
.switch.off { background: #D8D0C0; }
.switch::after { content: ""; position: absolute; width: 16px; height: 16px;
  border-radius: 50%; background: #fff; top: 2px; right: 2px; transition: all 0.18s ease; }
.switch.off::after { right: auto; left: 2px; }

/* Empty state */
.empty { display: flex; flex-direction: column; align-items: center; justify-content: center;
  text-align: center; padding: 36px 20px 60px; }
.empty .glyph { width: 64px; height: 64px; border-radius: 16px; background: var(--accent-soft);
  color: var(--accent); display: flex; align-items: center; justify-content: center; margin-bottom: 16px; }
.empty .glyph svg { width: 32px; height: 32px; }
.empty h3 { font-family: var(--font-serif); font-size: 17px; font-weight: 500; margin: 0 0 6px; }
.empty p { color: var(--ink-soft); margin: 0; max-width: 360px; line-height: 1.5; }
.empty .hints { display: flex; flex-direction: column; gap: 8px; margin-top: 20px; width: 100%; max-width: 380px; }
.empty .hint { display: flex; align-items: center; gap: 10px; background: var(--surface);
  border: 1px solid var(--line); border-radius: 8px; padding: 10px 13px; font-size: 12px;
  color: var(--ink-soft); text-align: left; }
.empty .hint .hic { width: 26px; height: 26px; border-radius: 6px; background: var(--accent-soft);
  color: var(--accent); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.empty .hint .hic svg { width: 14px; height: 14px; }
.empty .hint b { color: var(--ink); font-weight: 600; }

/* Sidebar polish */
.side-item .ic, .side-settings .ic { width: 15px; display: inline-flex; align-items: center;
  justify-content: center; opacity: 0.7; }
.side-item .ic svg, .side-settings .ic svg { width: 15px; height: 15px; }
.side-item.active .ic { opacity: 1; }
.totals { background: var(--bg); border: 1px solid var(--line); border-radius: 8px;
  padding: 10px 12px; margin-bottom: 6px; }
.totals .label { font-size: 9px; text-transform: uppercase; letter-spacing: 0.07em;
  color: var(--ink-soft); margin-bottom: 4px; }
.totals .spd { display: flex; justify-content: space-between; font-size: 12px; }

/* Settings sections */
.settings-group { margin-bottom: 18px; }
.settings-group .group-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.07em;
  color: var(--ink-soft); margin-bottom: 8px; }
.settings-row { display: flex; align-items: center; justify-content: space-between;
  padding: 9px 0; border-bottom: 1px solid var(--line); gap: 12px; }
.settings-row:last-child { border-bottom: 0; }
.settings-row .num { width: 90px; text-align: right; }
```

- [ ] **Step 2: Create `src/icons.js` with the full icon map**

```js
// Monochrome line-icon set. Each value is inline SVG markup using
// currentColor so the surrounding element controls the color.
// All icons share viewBox 0 0 24 24, stroke-width 2.

const SVG = (body) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${body}</svg>`;

export const ICONS = {
  // file-type categories
  video:    SVG('<rect x="2" y="5" width="20" height="14" rx="2"/><path d="m10 9 5 3-5 3z" fill="currentColor" stroke="none"/>'),
  audio:    SVG('<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>'),
  image:    SVG('<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-5-5L5 21"/>'),
  document: SVG('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>'),
  archive:  SVG('<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>'),
  program:  SVG('<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2"/>'),
  folder:   SVG('<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>'),
  other:    SVG('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>'),

  // sidebar filters
  all:         SVG('<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>'),
  downloading: SVG('<path d="M12 3v14"/><path d="m6 11 6 6 6-6"/><path d="M5 21h14"/>'),
  seeding:     SVG('<path d="M12 21V7"/><path d="m6 13 6-6 6 6"/><path d="M5 3h14"/>'),
  completed:   SVG('<path d="M20 6 9 17l-5-5"/>'),
  paused:      SVG('<rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/>'),
  gear:        SVG('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"/>'),

  // empty-state hint icons + Drift wave glyph
  link: SVG('<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>'),
  plus: SVG('<path d="M12 5v14"/><path d="M5 12h14"/>'),
  wave: SVG('<path d="M2 8c2 0 2 2 4 2s2-2 4-2 2 2 4 2 2-2 4-2 2 2 4 2"/><path d="M2 14c2 0 2 2 4 2s2-2 4-2 2 2 4 2 2-2 4-2 2 2 4 2"/>'),
};

// Map a filename to a file-type category key.
const EXT = {
  video: "mp4 mkv avi mov wmv flv webm m4v mpg mpeg ts m2ts".split(" "),
  audio: "mp3 flac wav aac ogg m4a wma opus alac".split(" "),
  document: "pdf epub mobi doc docx xls xlsx ppt pptx txt rtf csv".split(" "),
  archive: "zip rar 7z tar gz bz2 xz".split(" "),
  program: "exe msi dmg deb rpm apk appimage iso img".split(" "),
  image: "jpg jpeg png webp gif bmp svg tiff raw heic".split(" "),
};

export function extToCategory(filename) {
  const m = /\.([a-z0-9]+)$/i.exec(String(filename || ""));
  if (!m) return null; // no recognizable extension
  const ext = m[1].toLowerCase();
  for (const cat of ["video", "audio", "program", "archive", "document", "image"]) {
    if (EXT[cat].includes(ext)) return cat;
  }
  return "other";
}

export function icon(key) { return ICONS[key] || ICONS.other; }
```

- [ ] **Step 3: Verify nothing breaks**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/icons.js','utf8'); console.log('icons.js ok')"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
cd 'D:\Personal Project\Drift\src-tauri'; cargo build 2>&1 | Select-Object -Last 3
```
Expected: `icons.js ok`, and cargo build finishes (frontend changes don't affect Rust; this just confirms the workspace still compiles).

- [ ] **Step 4: Commit**

```bash
cd "/d/Personal Project/Drift"
git add src/styles.css src/icons.js
git commit -m "UI polish: add design tokens, icon module, and shared component CSS"
```

---

## Task 2: Torrent row redesign

**Files:**
- Modify: `src/main.js` (`rowHtml`, `renderExpanded`, add `fmtEta` + `iconForTorrent` helpers, import from icons.js)

- [ ] **Step 1: Import the icon module at the top of `src/main.js`**

The file currently starts with:
```js
import { invoke } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/core.js";
import { listen } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/event.js";
```
Add a third import line directly after them:
```js
import { ICONS, icon, extToCategory } from "./icons.js";
```

- [ ] **Step 2: Add helper functions** near the other helpers (after the `escape` function definition):

```js
// Pick a file-type icon for a torrent row. Rows only know the torrent name,
// so use its extension when present, else fall back to a folder icon.
function iconForTorrent(t) {
  const cat = extToCategory(t.name);
  return { key: cat ?? "folder", cat: cat ?? "folder" };
}

// Human ETA from remaining bytes / current speed.
function fmtEta(t) {
  if (!t.down_bps || t.down_bps <= 0) return "—";
  const remaining = Math.max(0, (t.total || 0) - (t.downloaded || 0));
  if (remaining === 0) return "—";
  let s = Math.round(remaining / t.down_bps);
  if (s < 60) return "<1m";
  const h = Math.floor(s / 3600); s -= h * 3600;
  const m = Math.floor(s / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

// Build the right-aligned meta line per state.
function metaLine(t) {
  if (t.state_label === "seeding" || t.state_label === "completed") {
    return `${fmtBytes(t.total)} · ↑ ${fmtBps(t.up_bps)} · ${t.peers} peers`;
  }
  if (t.state_label === "paused") {
    return `${fmtBytes(t.downloaded)} / ${fmtBytes(t.total)} · paused`;
  }
  // downloading / stalled / initializing
  return `${fmtBytes(t.downloaded)} / ${fmtBytes(t.total)} · ${fmtBps(t.down_bps)} · ${t.peers} peers · ETA ${fmtEta(t)}`;
}
```

- [ ] **Step 3: Replace `rowHtml`** with:

```js
function rowHtml(t) {
  const pct = t.total > 0 ? Math.floor(100 * t.downloaded / t.total) : 0;
  const ic = iconForTorrent(t);
  const stClass = "st-" + (t.state_label || "downloading");
  const stColorVar = `var(--st-${t.state_label || "downloading"}, var(--accent))`;
  const label = (t.state_label || "downloading").replace(/^\w/, c => c.toUpperCase());
  return `<div class="torrent-row" data-ih="${t.infohash}">
    <div class="row-grid">
      <div class="ficon ${ic.cat}">${icon(ic.key)}</div>
      <div>
        <div class="name">${escape(t.name)}</div>
        <div class="progress"><div style="width:${pct}%; background:${stColorVar}"></div></div>
        <div class="meta">${escape(metaLine(t))}</div>
      </div>
      <div class="right">
        <div class="pct">${pct}%</div>
        <div class="state ${stClass}"><span class="state-dot"></span>${label}</div>
      </div>
    </div>
    ${expanded.has(t.infohash) ? renderExpanded(t) : ""}
  </div>`;
}
```

- [ ] **Step 4: Update the `.row-grid` columns in `src/styles.css`**

Find `.row-grid { display: grid; grid-template-columns: 1fr 80px; … }` and change it to a 3-column grid with the icon:
```css
.row-grid { display: grid; grid-template-columns: auto 1fr 90px; column-gap: 12px; align-items: center; }
```
Find `.torrent-row` and add a hover rule right after it:
```css
.torrent-row:hover { background: rgba(0,0,0,0.02); }
```
Find `.progress { … background: var(--accent-soft); … }` and change the track to:
```css
.progress { height: 6px; background: var(--prog-track); border-radius: 99px; overflow: hidden; }
```
Find `.row-grid .right .state` and replace with:
```css
.row-grid .right .state { font-size: 11px; display: flex; align-items: center; justify-content: flex-end; }
```

- [ ] **Step 5: Restyle the expanded file list** — replace `renderExpanded` with a version that adds the per-file line icon:

```js
function renderExpanded(t) {
  const id = "exp-" + t.infohash;
  setTimeout(async () => {
    const el = document.getElementById(id);
    if (!el) return;
    try {
      const files = await invoke("torrent_files", { infohash: t.infohash });
      el.innerHTML = files.map(f => {
        const pct = f.size > 0 ? Math.floor(100 * f.downloaded / f.size) : 0;
        const cat = extToCategory(f.path) ?? "other";
        return `<div class="file-row">
          <label style="display:flex; gap:8px; align-items:center; flex:1; min-width:0;">
            <input type="checkbox" data-i="${f.index}" ${f.selected ? "checked" : ""}>
            <span class="ficon ${cat}" style="width:20px;height:20px;border-radius:5px">${icon(cat)}</span>
            <span style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap">${escape(f.path)}</span>
          </label>
          <span style="flex-shrink:0">${pct}% · ${fmtBytes(f.size)}</span>
        </div>`;
      }).join("");
      el.querySelectorAll("input[type=checkbox]").forEach(cb => cb.onchange = async () => {
        const sel = [...el.querySelectorAll("input[type=checkbox]:checked")].map(x => +x.dataset.i);
        try { await invoke("set_file_selection", { infohash: t.infohash, selected: sel }); }
        catch (e) { showToast("error", friendlyError(e)); cb.checked = !cb.checked; }
      });
    } catch (e) {
      el.innerHTML = `<div class="file-row">Couldn't load files: ${escape(String(e))}</div>`;
    }
  }, 0);
  return `<div class="expanded-files" id="${id}"><div class="file-row">Loading file list…</div></div>`;
}
```
Also update `.file-row svg { width:13px; height:13px; }` by appending to `src/styles.css`:
```css
.file-row .ficon svg { width: 12px; height: 12px; }
```

- [ ] **Step 6: Verify + commit**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/main.js','utf8'); console.log('main.js ok')"
```
```bash
cd "/d/Personal Project/Drift"
git add src/main.js src/styles.css
git commit -m "UI polish: redesign torrent rows with icons, state colors, and ETA"
```

---

## Task 3: Sidebar refinement

**Files:**
- Modify: `src/main.js` (`renderSidebar`)

- [ ] **Step 1: Replace `renderSidebar`** with the icon + totals-card version:

```js
function renderSidebar() {
  const el = document.getElementById("sidebar");
  const counts = Object.fromEntries(FILTERS.map(f => [f.key, 0]));
  for (const t of torrents) {
    counts.all++;
    if (counts[t.state_label] != null) counts[t.state_label]++;
  }
  let down = 0, up = 0;
  for (const t of torrents) { down += t.down_bps; up += t.up_bps; }
  const iconKey = { all: "all", downloading: "downloading", seeding: "seeding", completed: "completed", paused: "paused" };
  el.innerHTML = FILTERS.map(f => `
    <div class="side-item ${currentFilter === f.key ? 'active' : ''}" data-key="${f.key}">
      <span class="ic">${icon(iconKey[f.key])}</span>
      <span>${f.label}</span><span class="ct" style="margin-left:auto; font-size:11px; opacity:.8">${counts[f.key]}</span>
    </div>`).join("") +
    `<div class="side-bottom">
      <div class="totals">
        <div class="label">Total</div>
        <div class="spd"><span>↓ ${fmtBps(down)}</span><span style="color:var(--ink-soft)">↑ ${fmtBps(up)}</span></div>
      </div>
      <div class="side-settings" id="side-settings"><span class="ic">${icon("gear")}</span>Settings</div>
    </div>`;
  el.querySelectorAll(".side-item").forEach(n => n.onclick = () => {
    currentFilter = n.dataset.key;
    document.getElementById("main-title").textContent =
      FILTERS.find(x => x.key === currentFilter).label + " downloads";
    renderAll();
  });
  el.querySelector("#side-settings").onclick = () => toggleSettings();
}
```

- [ ] **Step 2: Update `.side-item` and `.side-settings` layout in `src/styles.css`**

Find `.side-item { … }` and ensure it has `gap: 9px` and aligns the icon. Replace the `.side-item` rule with:
```css
.side-item { padding: 8px 11px; border-radius: var(--radius-sm); color: var(--ink-soft); cursor: pointer;
  display: flex; align-items: center; gap: 9px; font-size: 13px; }
```
Replace `.side-settings` rule with:
```css
.side-settings { padding: 8px 11px; border-radius: var(--radius-sm); color: var(--ink-soft); cursor: pointer;
  font-size: 13px; display: flex; align-items: center; gap: 9px; }
```

- [ ] **Step 3: Verify + commit**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/main.js','utf8'); console.log('ok')"
```
```bash
cd "/d/Personal Project/Drift"
git add src/main.js src/styles.css
git commit -m "UI polish: sidebar filter icons and totals card"
```

---

## Task 4: Empty state

**Files:**
- Modify: `src/main.js` (`renderList` + new `emptyStateHtml`)

- [ ] **Step 1: Add `emptyStateHtml`** near the render functions:

```js
function emptyStateHtml() {
  if (currentFilter === "all") {
    return `<div class="empty">
      <div class="glyph">${icon("wave")}</div>
      <h3>Nothing downloading yet</h3>
      <p>Three ways to get started:</p>
      <div class="hints">
        <div class="hint"><span class="hic">${icon("link")}</span>
          <div><b>Paste a magnet link</b> — Drift watches your clipboard and offers to add it.</div></div>
        <div class="hint"><span class="hic">${icon("document")}</span>
          <div><b>Drop a .torrent file</b> — drag it anywhere onto this window.</div></div>
        <div class="hint"><span class="hic">${icon("plus")}</span>
          <div><b>Click Add torrent</b> — paste or browse in the dialog.</div></div>
      </div>
    </div>`;
  }
  const labels = { downloading: "downloading", seeding: "seeding", completed: "completed", paused: "paused" };
  return `<div class="empty">
    <div class="glyph">${icon("wave")}</div>
    <h3>Nothing ${labels[currentFilter] || "here"} right now</h3>
    <p>Torrents will show up here when they enter this state.</p>
  </div>`;
}
```

- [ ] **Step 2: Update `renderList`** to render the empty state when there are no matches:

```js
function renderList() {
  const list = document.getElementById("torrent-list");
  const filtered = currentFilter === "all" ? torrents
    : torrents.filter(t => t.state_label === currentFilter);
  if (filtered.length === 0) {
    list.innerHTML = emptyStateHtml();
    return;
  }
  list.innerHTML = filtered.map(t => rowHtml(t)).join("");
  list.querySelectorAll(".torrent-row").forEach(n => {
    n.onclick = () => { toggleExpand(n.dataset.ih); };
    n.oncontextmenu = (e) => { e.preventDefault(); openContextMenu(n.dataset.ih, e.clientX, e.clientY); };
  });
}
```

- [ ] **Step 3: Verify + commit**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/main.js','utf8'); console.log('ok')"
```
```bash
cd "/d/Personal Project/Drift"
git add src/main.js
git commit -m "UI polish: empty state with action hints + per-filter variant"
```

---

## Task 5: Settings panel — sections + toggle switches

**Files:**
- Modify: `src/main.js` (`toggleSettings`)

- [ ] **Step 1: Replace the body-building portion of `toggleSettings`.**

In `toggleSettings`, replace the `panel.innerHTML = \`…\`` assignment (the whole template through the closing backtick) with the grouped, switch-based version below. Keep the surrounding logic (the early-return toggle, `const cfg = await invoke("get_settings")`, `panel.classList.add("open")`, and the save/cancel handlers) — only the markup and the boolean-input wiring change.

```js
  panel.innerHTML = `
    <h2 style="font-family:var(--font-serif); margin:0 0 16px; font-size:18px; font-weight:500">Settings</h2>

    <div class="settings-group">
      <div class="group-label">Downloads</div>
      <div class="settings-row"><span>Default download folder</span></div>
      <input type="text" id="s-root" value="${escape(cfg.download_root)}" style="margin-bottom:10px">
      <div class="settings-row"><span>Download limit (KB/s)</span>
        <input class="num" type="text" id="s-down" value="${cfg.download_kbps}"></div>
      <div class="settings-row"><span>Upload limit (KB/s)</span>
        <input class="num" type="text" id="s-up" value="${cfg.upload_kbps}"></div>
    </div>

    <div class="settings-group">
      <div class="group-label">Behavior</div>
      <div class="settings-row"><span>Watch clipboard for magnet links</span>
        <button class="switch ${cfg.clipboard_watch ? "" : "off"}" id="s-clip" data-on="${cfg.clipboard_watch}"></button></div>
      <div class="settings-row"><span>Close button hides to tray</span>
        <button class="switch ${cfg.close_to_tray ? "" : "off"}" id="s-tray" data-on="${cfg.close_to_tray}"></button></div>
      <div class="settings-row"><span>Start with Windows</span>
        <button class="switch ${cfg.start_with_windows ? "" : "off"}" id="s-startup" data-on="${cfg.start_with_windows}"></button></div>
    </div>

    <details class="settings-group">
      <summary style="cursor:pointer; font-size:10px; text-transform:uppercase; letter-spacing:0.07em; color:var(--ink-soft)">Category extensions</summary>
      ${["video","audio","documents","compressed","programs","images"].map(k => `
        <div class="settings-row" style="flex-direction:column; align-items:stretch; gap:6px">
          <label style="font-size:12px; color:var(--ink-soft)">${k}</label>
          <input type="text" id="s-cat-${k}" value="${escape(cfg.category_map[k].join(' '))}">
        </div>`).join("")}
    </details>

    <div style="display:flex; justify-content:flex-end; gap:8px; margin-top:18px">
      <button class="btn-ghost" id="s-cancel">Close</button>
      <button class="btn-primary" id="s-save">Save</button>
    </div>`;
  panel.classList.add("open");

  // Toggle switches: clicking flips the data-on flag and the .off class.
  panel.querySelectorAll(".switch").forEach(sw => sw.onclick = () => {
    const on = sw.dataset.on !== "true";
    sw.dataset.on = String(on);
    sw.classList.toggle("off", !on);
  });

  document.getElementById("s-cancel").onclick = () => panel.classList.remove("open");
  document.getElementById("s-save").onclick = async () => {
    const isOn = id => document.getElementById(id).dataset.on === "true";
    const value = {
      download_root: document.getElementById("s-root").value,
      download_kbps: +document.getElementById("s-down").value || 0,
      upload_kbps: +document.getElementById("s-up").value || 0,
      clipboard_watch: isOn("s-clip"),
      close_to_tray: isOn("s-tray"),
      start_with_windows: isOn("s-startup"),
      category_map: Object.fromEntries(
        ["video","audio","documents","compressed","programs","images"].map(k =>
          [k, document.getElementById("s-cat-"+k).value.split(/\s+/).filter(Boolean)])),
    };
    try { await invoke("set_settings", { value }); panel.classList.remove("open"); showToast("info", "Settings saved."); }
    catch (e) { showToast("error", friendlyError(e)); }
  };
```

NOTE: this replaces BOTH the old `panel.innerHTML` block AND the old `s-cancel`/`s-save` handlers (the new versions are included above). Delete the old checkbox-based handlers so they don't double-bind.

- [ ] **Step 2: Verify + commit**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/main.js','utf8'); console.log('ok')"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
cd 'D:\Personal Project\Drift\src-tauri'; cargo build 2>&1 | Select-Object -Last 3
```
```bash
cd "/d/Personal Project/Drift"
git add src/main.js
git commit -m "UI polish: grouped settings with toggle switches"
```

---

## Task 6: Add-dialog file icons

**Files:**
- Modify: `src/main.js` (`renderMeta` inside `openAddDialog`)

- [ ] **Step 1: Update the file-row markup in `renderMeta`.**

Inside `openAddDialog`, find the `renderMeta` function's `filesHtml` builder:
```js
    const filesHtml = lastMeta.files.map((f, i) => `
      <div class="file-row">
        <label style="display:flex; gap:8px; align-items:center">
          <input type="checkbox" data-i="${i}" ${selected.includes(i) ? "checked" : ""}>
          <span>${escape(f.path)}</span>
        </label>
        <span>${fmtBytes(f.size)}</span>
      </div>`).join("");
```
Replace it with a version that adds the per-file line icon:
```js
    const filesHtml = lastMeta.files.map((f, i) => {
      const cat = extToCategory(f.path) ?? "other";
      return `
      <div class="file-row">
        <label style="display:flex; gap:8px; align-items:center; flex:1; min-width:0">
          <input type="checkbox" data-i="${i}" ${selected.includes(i) ? "checked" : ""}>
          <span class="ficon ${cat}" style="width:20px;height:20px;border-radius:5px">${icon(cat)}</span>
          <span style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap">${escape(f.path)}</span>
        </label>
        <span style="flex-shrink:0">${fmtBytes(f.size)}</span>
      </div>`;
    }).join("");
```

- [ ] **Step 2: Verify + commit**

```powershell
node -e "require('fs').readFileSync('D:/Personal Project/Drift/src/main.js','utf8'); console.log('ok')"
```
```bash
cd "/d/Personal Project/Drift"
git add src/main.js
git commit -m "UI polish: file-type icons in the Add Torrent dialog"
```

---

## Task 7: Version bump to 0.2.0 + smoke checklist + release build

**Files:**
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- Modify: `docs/smoke-checklist.md`

- [ ] **Step 1: Bump the version in all three manifests to `0.2.0`.**

- `package.json`: change `"version": "0.1.0"` → `"version": "0.2.0"`.
- `src-tauri/Cargo.toml`: under `[package]`, change `version = "0.1.0"` → `version = "0.2.0"`.
- `src-tauri/tauri.conf.json`: change the top-level `"version": "0.1.0"` → `"version": "0.2.0"`.

- [ ] **Step 2: Append a "UI polish (0.2.0)" section to `docs/smoke-checklist.md`:**

```markdown

## UI polish (0.2.0)
- [ ] Each state shows the correct dot color + tinted progress (downloading=coral, seeding=sage, completed=teal, paused=gray, stalled=amber)
- [ ] Torrent rows show a file-type line icon (or folder icon for folder torrents)
- [ ] ETA shows a sensible value while downloading, "—" when paused/stalled/0 speed
- [ ] Sidebar filters have icons + live counts; totals card shows aggregate down/up
- [ ] Empty "All" filter shows the wave glyph + three hint cards; other empty filters show the lighter variant
- [ ] Settings is grouped (Downloads / Behavior / Categories); the three booleans are toggle switches that flip coral/gray and persist after Save + reopen
- [ ] Add dialog file rows show per-file line icons; long save paths still front-truncate
- [ ] Row hover highlights; progress bars animate smoothly on the 1 Hz updates with no flicker
```

- [ ] **Step 3: Build the release bundle.**

```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
cd 'D:\Personal Project\Drift'
cargo tauri build 2>&1 | Select-Object -Last 6
```
Expected: produces `Drift_0.2.0_x64_en-US.msi` and `Drift_0.2.0_x64-setup.exe` under `src-tauri/target/release/bundle/`.

- [ ] **Step 4: Commit.**

```bash
cd "/d/Personal Project/Drift"
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/Cargo.lock docs/smoke-checklist.md
git commit -m "Bump version to 0.2.0 and add UI-polish smoke checklist"
```

---

## Done

After Task 7, Drift 0.2.0 is built with the full UI polish. The implementer should manually walk the new smoke-checklist section against `cargo tauri dev` before the build is considered shippable.
