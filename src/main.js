import { invoke } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/core.js";
import { listen } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/event.js";
import { getCurrentWindow } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/window.js";

// NOTE: production builds should vendor these via npm + a bundler. For
// development MVP, CDN imports keep the surface minimal. If CSP blocks them,
// switch to: import {invoke} from "@tauri-apps/api/core" with a bundler.

const win = getCurrentWindow();
document.getElementById("btn-min").onclick = () => win.minimize();
document.getElementById("btn-max").onclick = () => win.toggleMaximize();
document.getElementById("btn-close").onclick = () => win.close();

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
  el.innerHTML = FILTERS.map(f => `
    <div class="side-item ${currentFilter === f.key ? 'active' : ''}" data-key="${f.key}">
      <span>${f.label}</span><span>${counts[f.key]}</span>
    </div>`).join("") +
    `<div class="side-totals">
      <div class="label">Total</div>
      <div style="font-size:13px">↓ ${fmtBps(down)}</div>
      <div style="font-size:11px;color:var(--ink-soft)">↑ ${fmtBps(up)}</div>
    </div>`;
  el.querySelectorAll(".side-item").forEach(n => n.onclick = () => {
    currentFilter = n.dataset.key;
    document.getElementById("main-title").textContent =
      FILTERS.find(x => x.key === currentFilter).label + " downloads";
    renderAll();
  });
}

function renderList() {
  const list = document.getElementById("torrent-list");
  const filtered = currentFilter === "all" ? torrents
    : torrents.filter(t => t.state_label === currentFilter);
  list.innerHTML = filtered.map(t => rowHtml(t)).join("");
  list.querySelectorAll(".torrent-row").forEach(n => {
    n.onclick = () => { toggleExpand(n.dataset.ih); };
    n.oncontextmenu = (e) => { e.preventDefault(); openContextMenu(n.dataset.ih, e.clientX, e.clientY); };
  });
}

function rowHtml(t) {
  const pct = t.total > 0 ? Math.floor(100 * t.downloaded / t.total) : 0;
  return `<div class="torrent-row" data-ih="${t.infohash}">
    <div class="row-grid">
      <div>
        <div class="name">${escape(t.name)}</div>
        <div class="progress"><div style="width:${pct}%"></div></div>
        <div class="meta">${fmtBytes(t.downloaded)} / ${fmtBytes(t.total)} · ${fmtBps(t.down_bps)} · ${t.peers} peers</div>
      </div>
      <div class="right"><div class="pct">${pct}%</div><div class="state">${t.state_label}</div></div>
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
        return `<div class="file-row">
          <label style="display:flex; gap:8px; align-items:center; flex:1;">
            <input type="checkbox" data-i="${f.index}" ${f.selected ? "checked" : ""}>
            <span>${escape(f.path)}</span>
          </label>
          <span>${pct}% · ${fmtBytes(f.size)}</span>
        </div>`;
      }).join("");
      // Mid-download deselect: when any checkbox changes, send the new selection set
      el.querySelectorAll("input[type=checkbox]").forEach(cb => cb.onchange = async () => {
        const sel = [...el.querySelectorAll("input[type=checkbox]:checked")].map(x => +x.dataset.i);
        try { await invoke("set_file_selection", { infohash: t.infohash, selected: sel }); }
        catch (e) { showToast("error", String(e)); cb.checked = !cb.checked; }
      });
    } catch (e) {
      el.innerHTML = `<div class="file-row">Couldn't load files: ${e}</div>`;
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
function escape(s) { const d=document.createElement("div"); d.textContent=s; return d.innerHTML; }

window.drift = { renderAll };  // for console debugging

// Wire up Add button (modal opens in Task 11)
document.getElementById("btn-add").onclick = () => openAddDialog();
document.getElementById("btn-settings").onclick = () => toggleSettings();

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

  async function refresh() {
    const src = ta.value.trim();
    if (!src) { meta.textContent = ""; btn.disabled = true; return; }
    meta.textContent = "Fetching metadata…";
    btn.disabled = true;
    try {
      const m = await invoke("peek", { source: src });
      lastMeta = m;
      selected = m.files.map((_, i) => i);
      renderMeta();
      btn.disabled = false;
    } catch (e) {
      meta.textContent = "Couldn't read this torrent: " + e;
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
    meta.innerHTML = `
      <div style="color:var(--ink); margin-bottom:6px">${escape(lastMeta.name)}</div>
      <div>${fmtBytes(lastMeta.total_size)} total · ${lastMeta.files.length} file(s)</div>
      <div style="margin-top:8px">Will go to <code>${escape(lastMeta.predicted_save_path)}</code></div>
      <div style="margin-top:10px; max-height:200px; overflow-y:auto">${filesHtml}</div>`;
    meta.querySelectorAll("input[type=checkbox]").forEach(cb => cb.onchange = () => {
      const i = +cb.dataset.i;
      if (cb.checked) { if (!selected.includes(i)) selected.push(i); }
      else { selected = selected.filter(x => x !== i); }
      btn.disabled = selected.length === 0;
    });
  }

  ta.addEventListener("blur", refresh);
  ta.addEventListener("paste", () => setTimeout(refresh, 0));
  document.getElementById("addcancel").onclick = () => root.innerHTML = "";
  document.getElementById("addconfirm").onclick = async () => {
    try {
      const allSelected = lastMeta && selected.length === lastMeta.files.length;
      await invoke("add_torrent", {
        req: { source: ta.value.trim(),
               overridePath: null,
               selectedFiles: allSelected ? null : selected }
      });
      root.innerHTML = "";
      torrents = await invoke("snapshot");
      renderAll();
    } catch (e) {
      showToast("error", e === "already_added" ? "Already in your list." : String(e));
    }
  };

  if (initialSource) refresh();
}
async function toggleSettings() {
  const panel = document.getElementById("settings-panel");
  if (panel.classList.contains("open")) { panel.classList.remove("open"); return; }
  const cfg = await invoke("get_settings");
  panel.innerHTML = `
    <h2 style="font-family:var(--font-serif); margin:0 0 18px; font-size:18px; font-weight:500">Settings</h2>

    <div class="settings-field">
      <label>Default download folder</label>
      <input type="text" id="s-root" value="${escape(cfg.download_root)}">
    </div>

    <div class="settings-field">
      <label>Download limit (KB/s, 0 = unlimited)</label>
      <input type="text" id="s-down" value="${cfg.download_kbps}">
    </div>
    <div class="settings-field">
      <label>Upload limit (KB/s, 0 = unlimited)</label>
      <input type="text" id="s-up" value="${cfg.upload_kbps}">
    </div>

    <div class="settings-field">
      <label style="display:flex; gap:8px; align-items:center; text-transform:none; letter-spacing:0">
        <input type="checkbox" id="s-clip" ${cfg.clipboard_watch ? "checked" : ""}>
        Watch clipboard for magnet links
      </label>
    </div>
    <div class="settings-field">
      <label style="display:flex; gap:8px; align-items:center; text-transform:none; letter-spacing:0">
        <input type="checkbox" id="s-tray" ${cfg.close_to_tray ? "checked" : ""}>
        Close button hides to tray
      </label>
    </div>
    <div class="settings-field">
      <label style="display:flex; gap:8px; align-items:center; text-transform:none; letter-spacing:0">
        <input type="checkbox" id="s-startup" ${cfg.start_with_windows ? "checked" : ""}>
        Start with Windows
      </label>
    </div>

    <details style="margin-top:18px">
      <summary style="cursor:pointer; font-size:13px; color:var(--ink-soft)">Category extensions</summary>
      ${["video","audio","documents","compressed","programs","images"].map(k => `
        <div class="settings-field">
          <label>${k}</label>
          <input type="text" id="s-cat-${k}" value="${cfg.category_map[k].join(' ')}">
        </div>`).join("")}
    </details>

    <div style="display:flex; justify-content:flex-end; gap:8px; margin-top:18px">
      <button class="btn-ghost" id="s-cancel">Close</button>
      <button class="btn-primary" id="s-save">Save</button>
    </div>`;
  panel.classList.add("open");

  document.getElementById("s-cancel").onclick = () => panel.classList.remove("open");
  document.getElementById("s-save").onclick = async () => {
    const value = {
      download_root: document.getElementById("s-root").value,
      download_kbps: +document.getElementById("s-down").value || 0,
      upload_kbps: +document.getElementById("s-up").value || 0,
      clipboard_watch: document.getElementById("s-clip").checked,
      close_to_tray: document.getElementById("s-tray").checked,
      start_with_windows: document.getElementById("s-startup").checked,
      category_map: Object.fromEntries(
        ["video","audio","documents","compressed","programs","images"].map(k =>
          [k, document.getElementById("s-cat-"+k).value.split(/\s+/).filter(Boolean)])),
    };
    try { await invoke("set_settings", { value }); panel.classList.remove("open"); showToast("info", "Settings saved."); }
    catch (e) { showToast("error", String(e)); }
  };
}

function openContextMenu(ih, x, y) {
  closeContextMenu();
  const t = torrents.find(x => x.infohash === ih);
  if (!t) return;
  const menu = document.createElement("div");
  menu.className = "context-menu";
  menu.style.left = x + "px";
  menu.style.top = y + "px";
  const items = [
    t.state_label === "paused"
      ? { label: "Resume", fn: () => invoke("resume", { infohash: ih }) }
      : { label: "Pause",  fn: () => invoke("pause",  { infohash: ih }) },
    { label: "Open folder", fn: () => invoke("open_folder", { infohash: ih }) },
    { label: "Copy magnet", fn: () => invoke("copy_magnet", { infohash: ih }) },
    { label: "Remove", fn: () => invoke("remove", { infohash: ih, deleteFiles: false }) },
    { label: "Remove + delete files", fn: () => invoke("remove", { infohash: ih, deleteFiles: true }) },
  ];
  menu.innerHTML = items.map((it, i) => `<div class="item" data-i="${i}">${it.label}</div>`).join("");
  document.body.appendChild(menu);
  menu.querySelectorAll(".item").forEach(n => n.onclick = async () => {
    try { await items[+n.dataset.i].fn(); } catch (e) { showToast("error", String(e)); }
    closeContextMenu();
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

// Drag-drop on the whole window
document.addEventListener("dragover", e => e.preventDefault());
document.addEventListener("drop", async e => {
  e.preventDefault();
  const file = e.dataTransfer?.files?.[0];
  if (!file) return;
  // Tauri provides the file path on drop; use it directly:
  const path = file.path || file.name;
  openAddDialog(path);
});
