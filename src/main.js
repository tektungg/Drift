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
  // Files list populated by `invoke("torrent_files", {ih})` in Task 12.
  return `<div class="expanded-files" data-ih="${t.infohash}"><div class="file-row">Loading file list…</div></div>`;
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
function openAddDialog() { /* Task 11 */ }
function toggleSettings() { /* Task 16 */ }
function openContextMenu(ih, x, y) { /* Task 10 */ }

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
