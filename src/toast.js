// Tauri API via withGlobalTauri (tauri.conf.json) — no CDN, works offline,
// and keeps remote script execution out of the strict CSP.
const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;
import { icon } from "./icons.js";

// One small always-on-top window, two kinds of toast:
//   "magnet"   — interactive prompt: a magnet was copied, download it?
//   "complete" — passive notice: a download finished (Drift-styled instead of
//                a native Windows notification, which is attributed to
//                PowerShell in dev and doesn't match the app's look).
// A magnet prompt may replace a completion notice, but never the other way
// round — the prompt is interactive and must not be stomped by passive news.
const win = getCurrentWindow();
let mode = null;     // "magnet" | "complete" | null (hidden)
let pending = null;  // payload for the current mode
let autoDismissTimer = null;

document.getElementById("t-glyph").innerHTML = icon("wave");
const titleEl = document.getElementById("t-title");
const nameEl = document.getElementById("t-name");
const actionBtn = document.getElementById("t-action");

async function showToast(newMode, payload, title, actionLabel, timeoutMs) {
  mode = newMode;
  pending = payload;
  titleEl.textContent = title;
  nameEl.textContent = payload.name ?? "";
  actionBtn.textContent = actionLabel;
  await win.show();
  clearTimeout(autoDismissTimer);
  autoDismissTimer = setTimeout(hideToast, timeoutMs);
}

async function hideToast() {
  clearTimeout(autoDismissTimer);
  mode = null;
  pending = null;
  await win.hide();
}

listen("magnet-detected", (e) => {
  // Show the magnet's display name (or truncated infohash if unnamed) so the
  // user can tell which torrent they're being asked about.
  showToast("magnet", e.payload, "Magnet link detected — download it?", "Yes", 10000);
});

listen("download-complete", (e) => {
  if (mode === "magnet") return; // never stomp an interactive prompt
  showToast("complete", e.payload, "Download complete", "Open folder", 6000);
});

document.getElementById("t-dismiss").onclick = () => hideToast();

actionBtn.onclick = async () => {
  if (!pending) return;
  const m = mode, p = pending;
  await hideToast();
  try {
    if (m === "magnet") {
      // Bring the main window forward, then ask main.js to open the Add
      // Torrent dialog with this magnet pre-filled. The dialog will fetch
      // metadata, show the file list, and let the user confirm — same flow
      // as if they'd pasted the magnet themselves.
      await invoke("focus_main");
      await emit("open-source", p.uri);
    } else {
      await invoke("open_folder", { infohash: p.infohash });
    }
  } catch (e) {
    await emit("toast", { kind: "error", message: `Couldn't complete that: ${e}` });
  }
};
