import { invoke } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/core.js";
import { listen } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/event.js";
import { icon, extToCategory } from "./icons.js";
import { filterTorrents, sortTorrents } from "./list-ops.js";

// NOTE: production builds should vendor these via npm + a bundler. For
// development MVP, CDN imports keep the surface minimal. If CSP blocks them,
// switch to: import {invoke} from "@tauri-apps/api/core" with a bundler.
//
// The window uses native Windows decorations (titlebar with min/max/close),
// so we no longer wire those buttons from JS.

const FILTERS = [
  { key: "all", label: "All" },
  { key: "downloading", label: "Downloading" },
  { key: "seeding", label: "Seeding" },
  { key: "completed", label: "Completed" },
  { key: "paused", label: "Paused" },
];
let currentFilter = "all";
let searchQuery = "";
let sortKey = localStorage.getItem("drift-sort-key") || "added";
let sortDir = localStorage.getItem("drift-sort-dir") || "desc";
let torrents = [];  // [{infohash, name, downloaded, total, down_bps, up_bps, peers, state_label}]
let expanded = new Set();
let selected = new Set();      // infohashes currently selected
let lastClickedIh = null;      // anchor for shift-range selection

// ── Theme (system / light / dark) ──
let themeChoice = "system";
function resolveTheme(choice) {
  if (choice === "dark") return "dark";
  if (choice === "light") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
function applyTheme(choice) {
  themeChoice = choice || "system";
  document.documentElement.dataset.theme = resolveTheme(themeChoice);
  try { localStorage.setItem("drift-theme", themeChoice); } catch (e) {}
}
// Track the OS theme live while in "system" mode.
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if (themeChoice === "system") document.documentElement.dataset.theme = resolveTheme("system");
});

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

function renderList() {
  const list = document.getElementById("torrent-list");
  const filtered = sortTorrents(
    filterTorrents(torrents, currentFilter, searchQuery),
    sortKey, sortDir
  );
  if (filtered.length === 0) {
    list.innerHTML = emptyStateHtml();
    return;
  }
  list.innerHTML = filtered.map(t => rowHtml(t)).join("");
  list.querySelectorAll(".torrent-row").forEach(n => {
    // Click-to-expand is scoped to the header (.row-grid) only — otherwise a
    // click on a file checkbox in the expanded area would bubble up and
    // collapse the row. Right-click anywhere on the row opens the menu.
    const grid = n.querySelector(".row-grid");
    if (grid) grid.onclick = (e) => {
      const ih = n.dataset.ih;
      if (e.ctrlKey || e.metaKey) {
        if (selected.has(ih)) selected.delete(ih); else selected.add(ih);
        lastClickedIh = ih;
        renderList(); updateBulkBar();
      } else if (e.shiftKey && lastClickedIh) {
        const order = [...document.querySelectorAll(".torrent-row")].map(r => r.dataset.ih);
        const i = order.indexOf(lastClickedIh), j = order.indexOf(ih);
        if (i !== -1 && j !== -1) {
          const [lo, hi] = i < j ? [i, j] : [j, i];
          for (let k = lo; k <= hi; k++) selected.add(order[k]);
        }
        renderList(); updateBulkBar();
      } else {
        toggleExpand(ih);
      }
    };
    n.oncontextmenu = (e) => { e.preventDefault(); openContextMenu(n.dataset.ih, e.clientX, e.clientY); };
  });
}

// Apply a 1 Hz progress update. A change to state_label alters the sidebar
// counts and filter membership, so that case needs a structural re-render;
// the common numeric-only tick patches the existing DOM in place to avoid the
// flicker (and lost hover/focus) of rebuilding the whole list every second.
function applyProgress(u) {
  const existing = torrents.find(t => t.infohash === u.infohash);
  if (!existing) {
    torrents.push({ name: u.infohash, ...u });
    renderAll();
    return;
  }
  const prevLabel = existing.state_label;
  Object.assign(existing, u);
  if (u.state_label !== prevLabel) { renderAll(); return; }
  patchRow(existing);
}

// Surgically update the dynamic bits of a single row without recreating nodes.
function patchRow(t) {
  const row = document.querySelector(`.torrent-row[data-ih="${t.infohash}"]`);
  if (!row) return;  // filtered out of the current view — nothing to patch
  const pct = t.total > 0 ? Math.floor(100 * t.downloaded / t.total) : 0;
  const bar = row.querySelector(".progress > div");
  if (bar) bar.style.width = pct + "%";          // CSS transition animates it
  const pctEl = row.querySelector(".right .pct");
  if (pctEl) pctEl.textContent = pct + "%";
  const metaEl = row.querySelector(".meta");
  if (metaEl) metaEl.textContent = metaLine(t);
  const det = row.querySelector(".exp-detail");
  if (det) {                                       // row is expanded
    det.innerHTML = detailLine(t);
    updateFileRows(t);
  }
}

// Refresh just the per-file percentages of an expanded row's file list in
// place — without rebuilding it — so checkboxes, hover and scroll position
// survive the 1 Hz tick.
async function updateFileRows(t) {
  const cont = document.getElementById("expf-" + t.infohash);
  if (!cont) return;
  const rows = cont.querySelectorAll(".file-row[data-fi]");
  if (rows.length === 0) return;  // still loading, or load failed
  try {
    const files = await invoke("torrent_files", { infohash: t.infohash });
    const byIdx = new Map(files.map(f => [f.index, f]));
    rows.forEach(r => {
      const f = byIdx.get(+r.dataset.fi);
      if (!f) return;
      const span = r.querySelector(".file-pct");
      if (span) {
        const p = f.size > 0 ? Math.floor(100 * f.downloaded / f.size) : 0;
        span.textContent = `${p}% · ${fmtBytes(f.size)}`;
      }
    });
  } catch (e) { /* transient — next tick will retry */ }
}

function rowHtml(t) {
  const pct = t.total > 0 ? Math.floor(100 * t.downloaded / t.total) : 0;
  const ic = iconForTorrent(t);
  const stClass = "st-" + (t.state_label || "downloading");
  const stColorVar = `var(--st-${t.state_label || "downloading"}, var(--accent))`;
  const label = (t.state_label || "downloading").replace(/^\w/, c => c.toUpperCase());
  return `<div class="torrent-row ${selected.has(t.infohash) ? "selected" : ""}" data-ih="${t.infohash}">
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

// Detail line shown at the top of an expanded row.
function detailLine(t) {
  const dl = t.downloaded || 0, up = t.uploaded || 0;
  const ratio = dl > 0 ? (up / dl).toFixed(2) : "0.00";
  const added = t.added_at
    ? new Date(t.added_at).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })
    : "—";
  const eta = (t.state_label === "downloading" || t.state_label === "stalled") ? fmtEta(t) : "—";
  return `${t.peers || 0} peers · ratio ${ratio} · ↑ ${fmtBytes(up)} · added ${escape(added)} · ETA ${eta}`;
}

function renderExpanded(t) {
  const id = "exp-" + t.infohash;
  const filesId = "expf-" + t.infohash;
  setTimeout(async () => {
    const el = document.getElementById(filesId);
    if (!el) return;
    try {
      const files = await invoke("torrent_files", { infohash: t.infohash });
      el.innerHTML = files.map(f => {
        const pct = f.size > 0 ? Math.floor(100 * f.downloaded / f.size) : 0;
        const cat = extToCategory(f.path) ?? "other";
        return `<div class="file-row" data-fi="${f.index}">
          <label style="display:flex; gap:8px; align-items:center; flex:1; min-width:0;">
            <input type="checkbox" data-i="${f.index}" ${f.selected ? "checked" : ""}>
            <span class="ficon ${cat}" style="width:20px;height:20px;border-radius:5px">${icon(cat)}</span>
            <span style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap">${escape(f.path)}</span>
          </label>
          <span class="file-pct" style="flex-shrink:0">${pct}% · ${fmtBytes(f.size)}</span>
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
  return `<div class="expanded-files" id="${id}">
    <div class="exp-detail">${detailLine(t)}</div>
    <div id="${filesId}"><div class="file-row">Loading file list…</div></div>
  </div>`;
}

function toggleExpand(ih) {
  if (expanded.has(ih)) expanded.delete(ih); else expanded.add(ih);
  renderList();
}

function renderAll() {
  for (const ih of [...selected]) if (!torrents.some(t => t.infohash === ih)) selected.delete(ih);
  renderSidebar(); renderList();
}

function updateBulkBar() {
  const bar = document.getElementById("bulk-bar");
  if (!bar) return;
  if (selected.size === 0) { bar.hidden = true; bar.innerHTML = ""; return; }
  bar.hidden = false;
  bar.innerHTML = `
    <span class="count">${selected.size} selected</span>
    <div class="spacer"></div>
    <button class="btn-ghost" id="bulk-resume">Resume</button>
    <button class="btn-ghost" id="bulk-pause">Pause</button>
    <button class="btn-ghost" id="bulk-remove">Remove</button>
    <button class="btn-ghost" id="bulk-clear">Clear</button>`;
  document.getElementById("bulk-resume").onclick = () => bulkAction("resume");
  document.getElementById("bulk-pause").onclick  = () => bulkAction("pause");
  document.getElementById("bulk-remove").onclick = () => bulkRemove();
  document.getElementById("bulk-clear").onclick  = () => {
    selected.clear(); lastClickedIh = null; renderList(); updateBulkBar();
  };
}

async function bulkAction(cmd) {
  const ids = [...selected];
  for (const ih of ids) {
    try { await invoke(cmd, { infohash: ih }); } catch (e) { /* no-op if not applicable */ }
  }
  torrents = await invoke("snapshot");
  selected.clear(); lastClickedIh = null;
  renderAll(); updateBulkBar();
}

async function bulkRemove() {
  const ids = [...selected];
  if (!ids.length) return;
  if (!window.confirm(`Remove ${ids.length} torrent(s) from Drift?`)) return;
  const deleteFiles = window.confirm(
    "Also DELETE the downloaded files from disk?\n\nOK = delete files.\nCancel = keep files on disk."
  );
  for (const ih of ids) {
    try { await invoke("remove", { infohash: ih, deleteFiles }); } catch (e) { /* ignore */ }
  }
  torrents = await invoke("snapshot");
  selected.clear(); lastClickedIh = null;
  renderAll(); updateBulkBar();
}

function wireListControls() {
  const search = document.getElementById("list-search");
  const sortSel = document.getElementById("list-sort");
  const dirBtn = document.getElementById("list-sortdir");
  if (!search || !sortSel || !dirBtn) return;
  sortSel.value = sortKey;
  dirBtn.textContent = sortDir === "asc" ? "↑" : "↓";
  search.addEventListener("input", () => { searchQuery = search.value; renderList(); });
  sortSel.addEventListener("change", () => {
    sortKey = sortSel.value; localStorage.setItem("drift-sort-key", sortKey); renderList();
  });
  dirBtn.addEventListener("click", () => {
    sortDir = sortDir === "asc" ? "desc" : "asc";
    localStorage.setItem("drift-sort-dir", sortDir);
    dirBtn.textContent = sortDir === "asc" ? "↑" : "↓";
    renderList();
  });
}

function fmtBytes(n) {
  if (n < 1024) return `${n} B`;
  const units = ["KB","MB","GB","TB"]; let v = n/1024; let i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 ? 2 : 1)} ${units[i]}`;
}
function fmtBps(n) { return `${fmtBytes(n)}/s`; }
// HTML-escape for BOTH body content AND attribute values. textContent->innerHTML
// only escapes &, <, > — leaving " and ' intact, which would break out of
// `value="${escape(x)}"` attribute contexts. Add explicit quote escaping.
function escape(s) {
  return String(s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

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
  return `${fmtBytes(t.downloaded)} / ${fmtBytes(t.total)} · ${fmtBps(t.down_bps)} · ${t.peers} peers · ETA ${fmtEta(t)}`;
}

function friendlyError(e) {
  const s = String(e);
  if (s.includes("already_added")) return "Already in your list.";
  if (s.includes("select_at_least_one")) return "Pick at least one file — to stop downloading entirely, remove the torrent.";
  if (s.includes("metadata_timeout")) return "Couldn't fetch metadata in 60s — too few seeds or no network.";
  if (s.toLowerCase().includes("not a magnet")) return "Couldn't read this torrent.";
  if (s.toLowerCase().includes("cannot read torrent file")) return "Couldn't read this torrent — paste a magnet link or drop a valid .torrent file.";
  if (s.includes("os error 112") || s.toLowerCase().includes("no space")) return "Disk full — torrent paused.";
  if (s.toLowerCase().includes("permission denied")) return "Write permission denied — torrent paused.";
  if (s.toLowerCase().includes("not_found")) return "Torrent not found.";
  return s;
}

window.drift = { renderAll };  // for console debugging

// Wire up Add button (modal opens in Task 11)
document.getElementById("btn-add").onclick = () => openAddDialog();
// Settings is wired per-render from inside renderSidebar() (#side-settings).

// Stubs filled by later tasks:
async function openAddDialog(initialSource = "") {
  const root = document.getElementById("modal-root");
  root.innerHTML = `
    <div class="modal-backdrop">
      <div class="modal">
        <h2>Add torrent</h2>
        <textarea id="addsrc" rows="3" style="resize:none" placeholder="Paste a magnet link or drop a .torrent file"></textarea>
        <div style="margin-top:8px">
          <button class="btn-ghost" id="addbrowse" style="font-size:12px; padding:5px 11px">Browse for a .torrent file…</button>
        </div>
        <div id="addmeta" style="margin-top:14px; min-height:80px; font-size:13px; color:var(--ink-soft)"></div>
        <div style="display:flex; justify-content:flex-end; gap:8px; margin-top:16px">
          <button class="btn-ghost" id="addcancel">Cancel</button>
          <button class="btn-primary" id="addconfirm" disabled>Add</button>
        </div>
      </div>
    </div>`;
  const ta = document.getElementById("addsrc");
  ta.value = initialSource;
  const meta = document.getElementById("addmeta");
  const btn = document.getElementById("addconfirm");
  let lastMeta = null;
  let selected = null;
  let lastFetched = "";  // prevents re-peek on the same source
  let fetchTimer = null;
  let overridePath = null;  // null = use auto-categorized path from peek

  async function refresh() {
    const src = ta.value.trim();
    if (!src) { meta.textContent = ""; btn.disabled = true; lastFetched = ""; return; }
    if (src === lastFetched) return;  // already have metadata for this exact source
    lastFetched = src;
    meta.textContent = "Fetching metadata…";
    btn.disabled = true;
    try {
      const m = await invoke("peek", { source: src });
      if (ta.value.trim() !== src) return;  // user changed source mid-fetch
      lastMeta = m;
      selected = m.files.map((_, i) => i);
      renderMeta();
      btn.disabled = false;
    } catch (e) {
      // Wrap + clamp to 5 lines so a long/garbage paste echoed in the error
      // can't overflow the dialog horizontally or blow up its height.
      meta.innerHTML = `<div class="meta-err">${escape(friendlyError(e))}</div>`;
      lastFetched = "";  // allow retry on the same string after a failure
    }
  }

  function renderMeta() {
    if (!lastMeta) return;
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
    const displayPath = overridePath ?? lastMeta.predicted_save_path;
    const customLabel = overridePath ? " (custom)" : "";
    meta.innerHTML = `
      <div style="color:var(--ink); margin-bottom:6px">${escape(lastMeta.name)}</div>
      <div>${fmtBytes(lastMeta.total_size)} total · ${lastMeta.files.length} file(s)</div>
      <div style="margin-top:8px; display:flex; align-items:center; gap:8px">
        <span style="flex-shrink:0">Will go to</span>
        <code class="path-trunc" title="${escape(displayPath)}"><bdi>${escape(displayPath)}</bdi></code>
        <button class="btn-ghost" id="change-folder"
                style="padding:3px 10px; font-size:11px; flex-shrink:0">Change…</button>
        ${overridePath ? `<button class="btn-ghost" id="reset-folder"
                style="padding:3px 10px; font-size:11px; flex-shrink:0">Reset</button>` : ""}
      </div>
      <div style="font-size:11px; color:var(--ink-soft); margin-top:2px">
        ${customLabel ? `Custom location · ` : `Auto-categorized · `}you can change it here or in Settings.
      </div>
      <div style="margin-top:10px; max-height:200px; overflow-y:auto">${filesHtml}</div>`;
    meta.querySelectorAll("input[type=checkbox]").forEach(cb => cb.onchange = () => {
      const i = +cb.dataset.i;
      if (cb.checked) { if (!selected.includes(i)) selected.push(i); }
      else { selected = selected.filter(x => x !== i); }
      btn.disabled = selected.length === 0;
    });
    document.getElementById("change-folder").onclick = async () => {
      const startDir = overridePath ?? lastMeta.predicted_save_path;
      try {
        const picked = await invoke("pick_folder", { start: startDir });
        if (picked) {
          overridePath = picked;
          renderMeta();
        }
      } catch (e) {
        showToast("error", friendlyError(e));
      }
    };
    const resetBtn = document.getElementById("reset-folder");
    if (resetBtn) resetBtn.onclick = () => { overridePath = null; renderMeta(); };
  }

  // Refresh strategy: peek immediately on paste, and after the user stops typing
  // for 500ms. Never on blur — clicking Add or dragging the window must not retrigger.
  ta.addEventListener("paste", () => setTimeout(refresh, 0));
  ta.addEventListener("input", () => {
    // The source changed; any cached metadata is no longer valid. Disable Add
    // until refresh() completes so the user can't fire add_torrent with stale
    // file selections that don't match the new torrent. Also clear any custom
    // save path the user picked — it was tied to the previous magnet.
    lastMeta = null;
    selected = null;
    overridePath = null;
    btn.disabled = true;
    clearTimeout(fetchTimer);
    fetchTimer = setTimeout(refresh, 500);
  });
  document.getElementById("addbrowse").onclick = async () => {
    try {
      const p = await invoke("pick_torrent_file");
      if (p) { ta.value = p; refresh(); }
    } catch (e) { showToast("error", friendlyError(e)); }
  };
  document.getElementById("addcancel").onclick = () => root.innerHTML = "";
  document.getElementById("addconfirm").onclick = async () => {
    const original = btn.textContent;
    btn.disabled = true;
    btn.textContent = "Adding…";
    try {
      const allSelected = lastMeta && selected.length === lastMeta.files.length;
      await invoke("add_torrent", {
        req: { source: ta.value.trim(),
               overridePath: overridePath,
               selectedFiles: allSelected ? null : selected }
      });
      root.innerHTML = "";
      torrents = await invoke("snapshot");
      renderAll();
    } catch (e) {
      btn.disabled = false;
      btn.textContent = original;
      showToast("error", friendlyError(e));
    }
  };

  if (initialSource) refresh();
}
async function toggleSettings() {
  const panel = document.getElementById("settings-panel");
  if (panel.classList.contains("open")) { panel.classList.remove("open"); return; }
  const cfg = await invoke("get_settings");
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
      <div class="group-label">Appearance</div>
      <div class="settings-row"><span>Theme</span>
        <div class="seg" id="s-theme">
          <button class="seg-btn ${(cfg.theme || 'system') === 'system' ? 'active' : ''}" data-val="system" title="System">${icon('system')}</button>
          <button class="seg-btn ${cfg.theme === 'light' ? 'active' : ''}" data-val="light" title="Light">${icon('light')}</button>
          <button class="seg-btn ${cfg.theme === 'dark' ? 'active' : ''}" data-val="dark" title="Dark">${icon('dark')}</button>
        </div>
      </div>
    </div>

    <div class="settings-group">
      <div class="group-label">Behavior</div>
      <div class="settings-row"><span>Watch clipboard for magnet links</span>
        <button class="switch ${cfg.clipboard_watch ? "" : "off"}" id="s-clip" data-on="${cfg.clipboard_watch}"></button></div>
      <div class="settings-row"><span>Close button hides to tray</span>
        <button class="switch ${cfg.close_to_tray ? "" : "off"}" id="s-tray" data-on="${cfg.close_to_tray}"></button></div>
      <div class="settings-row"><span>Start with Windows</span>
        <button class="switch ${cfg.start_with_windows ? "" : "off"}" id="s-startup" data-on="${cfg.start_with_windows}"></button></div>
      <div class="settings-row"><span>Open magnet links with Drift</span>
        <button class="switch ${cfg.magnet_handler ? "" : "off"}" id="s-magnet" data-on="${!!cfg.magnet_handler}"></button></div>
    </div>

    <div class="settings-group">
      <div class="group-label">Queue</div>
      <div class="settings-row"><span>Max active downloads (0 = unlimited)</span>
        <input class="num" type="text" id="s-maxactive" value="${cfg.max_active_downloads ?? 3}"></div>
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

  // Theme segmented control: mark active + apply live for instant preview.
  const themeBtns = panel.querySelectorAll("#s-theme .seg-btn");
  themeBtns.forEach(b => b.onclick = () => {
    themeBtns.forEach(x => x.classList.remove("active"));
    b.classList.add("active");
    applyTheme(b.dataset.val);
  });

  document.getElementById("s-cancel").onclick = () => panel.classList.remove("open");
  document.getElementById("s-save").onclick = async () => {
    const isOn = id => document.getElementById(id).dataset.on === "true";
    const themeBtn = panel.querySelector("#s-theme .seg-btn.active");
    const value = {
      download_root: document.getElementById("s-root").value,
      download_kbps: +document.getElementById("s-down").value || 0,
      upload_kbps: +document.getElementById("s-up").value || 0,
      clipboard_watch: isOn("s-clip"),
      close_to_tray: isOn("s-tray"),
      start_with_windows: isOn("s-startup"),
      magnet_handler: isOn("s-magnet"),
      max_active_downloads: +document.getElementById("s-maxactive").value || 0,
      theme: themeBtn ? themeBtn.dataset.val : "system",
      category_map: Object.fromEntries(
        ["video","audio","documents","compressed","programs","images"].map(k =>
          [k, document.getElementById("s-cat-"+k).value.split(/\s+/).filter(Boolean)])),
    };
    try {
      await invoke("set_settings", { value });
      applyTheme(value.theme);  // commit the chosen theme + sync localStorage
      panel.classList.remove("open");
      showToast("info", "Settings saved.");
    } catch (e) { showToast("error", friendlyError(e)); }
  };
}

function openContextMenu(ih, x, y) {
  closeContextMenu();
  const t = torrents.find(it => it.infohash === ih);
  if (!t) return;
  const menu = document.createElement("div");
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";
  // Each item has: label, fn (the async action), and onSuccess (optional
  // user-visible feedback after the action completes).
  const items = [
    t.state_label === "paused"
      ? { label: "Resume", fn: () => invoke("resume", { infohash: ih }) }
      : { label: "Pause",  fn: () => invoke("pause",  { infohash: ih }) },
    { label: "Open folder", fn: () => invoke("open_folder", { infohash: ih }) },
    { label: "Copy magnet",
      fn: () => invoke("copy_magnet", { infohash: ih }),
      onSuccess: () => showToast("info", "Magnet copied to clipboard.") },
    { label: "Remove", fn: () => invoke("remove", { infohash: ih, deleteFiles: false }) },
    { label: "Remove + delete files", fn: () => invoke("remove", { infohash: ih, deleteFiles: true }) },
  ];
  menu.innerHTML = items.map((it, i) => `<div class="item" data-i="${i}">${escape(it.label)}</div>`).join("");
  document.body.appendChild(menu);
  menu.querySelectorAll(".item").forEach(n => n.onclick = async () => {
    const item = items[+n.dataset.i];
    closeContextMenu();
    try {
      await item.fn();
      if (item.onSuccess) item.onSuccess();
      // Always re-sync from the backend after a context action so that
      // removed rows disappear immediately and paused/resumed states show.
      torrents = await invoke("snapshot");
      renderAll();
    } catch (e) {
      showToast("error", friendlyError(e));
    }
  });
  document.addEventListener("click", closeContextMenu, { once: true });
}
function closeContextMenu() { document.querySelectorAll(".context-menu").forEach(n => n.remove()); }

function showToast(kind, message) {
  const stack = document.getElementById("toasts");
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.textContent = message;
  stack.appendChild(el);
  setTimeout(() => el.remove(), 5000);
}

// listen for toast events from Rust
listen("toast", (e) => showToast(e.payload.kind, e.payload.message));

// Single-instance: second launch forwards magnet/torrent to open the Add dialog.
listen("open-source", (e) => openAddDialog(e.payload));

// Reliable fallback for the already-running case: when the window is brought
// forward by a magnet handoff (single-instance calls set_focus), pull any
// pending source. take_pending_source() returns null when there's nothing,
// so a normal focus is a harmless no-op.
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && selected.size) {
    selected.clear(); lastClickedIh = null; renderList(); updateBulkBar();
  }
});

window.addEventListener("focus", async () => {
  try {
    const pending = await invoke("take_pending_source");
    if (pending) openAddDialog(pending);
  } catch (e) { /* nothing pending */ }
});

// Boot: load current snapshot from Rust + subscribe to events
(async () => {
  // Reconcile the persisted theme (source of truth) with the pre-paint guess.
  try {
    const cfg = await invoke("get_settings");
    applyTheme(cfg.theme || "system");
  } catch (e) { /* pre-paint script already applied a sensible default */ }
  try {
    torrents = await invoke("snapshot");
  } catch (e) {
    console.error("snapshot failed", e);
    torrents = [];
  }
  renderAll();
  wireListControls();
  await listen("progress", (e) => applyProgress(e.payload));
  // If Drift was cold-launched from a magnet/.torrent (e.g. a magnet clicked
  // in a browser), the source was stashed in Rust — pull it now that our
  // listeners and dialog are ready, and open the Add dialog pre-filled.
  try {
    const pending = await invoke("take_pending_source");
    if (pending) openAddDialog(pending);
  } catch (e) { /* no pending source */ }
})();

// Drag-drop is handled entirely in Rust via WindowEvent::DragDrop, which
// re-emits the dropped path as our existing `open-source` event. See main.rs.
// (We can't reliably listen() to `tauri://drag-drop` here because Tauri 2 emits
//  it to a window-scoped EventTarget that the CDN-loaded event module doesn't
//  always pick up across minor versions.)
