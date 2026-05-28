import { invoke } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/core.js";
import { listen } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/event.js";
import { ICONS, icon, extToCategory } from "./icons.js";

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
let torrents = [];  // [{infohash, name, downloaded, total, down_bps, up_bps, peers, state_label}]
let expanded = new Set();

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

function toggleExpand(ih) {
  if (expanded.has(ih)) expanded.delete(ih); else expanded.add(ih);
  renderList();
}

function renderAll() { renderSidebar(); renderList(); }

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
        <textarea id="addsrc" rows="3" placeholder="Paste a magnet link or drop a .torrent file"></textarea>
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
      meta.textContent = friendlyError(e);
      lastFetched = "";  // allow retry on the same string after a failure
    }
  }

  function renderMeta() {
    if (!lastMeta) return;
    const filesHtml = lastMeta.files.map((f, i) => `
      <div class="file-row">
        <label style="display:flex; gap:8px; align-items:center">
          <input type="checkbox" data-i="${i}" ${selected.includes(i) ? "checked" : ""}>
          <span>${escape(f.path)}</span>
        </label>
        <span>${fmtBytes(f.size)}</span>
      </div>`).join("");
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

// Single-instance: second launch forwards magnet/torrent to open the Add dialog
listen("open-source", (e) => openAddDialog(e.payload));

// Boot: load current snapshot from Rust + subscribe to events
(async () => {
  try {
    torrents = await invoke("snapshot");
  } catch (e) {
    console.error("snapshot failed", e);
    torrents = [];
  }
  renderAll();
  await listen("progress", (e) => {
    const u = e.payload;
    const existing = torrents.find(t => t.infohash === u.infohash);
    if (existing) Object.assign(existing, u);
    else torrents.push({ name: u.infohash, ...u });
    renderAll();
  });
})();

// Drag-drop is handled entirely in Rust via WindowEvent::DragDrop, which
// re-emits the dropped path as our existing `open-source` event. See main.rs.
// (We can't reliably listen() to `tauri://drag-drop` here because Tauri 2 emits
//  it to a window-scoped EventTarget that the CDN-loaded event module doesn't
//  always pick up across minor versions.)
