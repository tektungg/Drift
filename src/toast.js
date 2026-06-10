// Tauri API via withGlobalTauri (tauri.conf.json) — no CDN, works offline,
// and keeps remote script execution out of the strict CSP.
const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const win = getCurrentWindow();
let pending = null;
let autoDismissTimer = null;

listen("magnet-detected", async (e) => {
  pending = e.payload;
  // Show the magnet's display name (or truncated infohash if unnamed) as a
  // small subtitle underneath the prompt so the user can tell which torrent
  // they're being asked about.
  document.getElementById("t-name").textContent = pending.name ?? "";
  await win.show();
  // 10s auto-dismiss
  clearTimeout(autoDismissTimer);
  autoDismissTimer = setTimeout(() => {
    if (pending) { win.hide(); pending = null; }
  }, 10000);
});

document.getElementById("t-dismiss").onclick = async () => {
  clearTimeout(autoDismissTimer);
  pending = null;
  await win.hide();
};

document.getElementById("t-add").onclick = async () => {
  if (!pending) return;
  const uri = pending.uri;
  clearTimeout(autoDismissTimer);
  pending = null;
  await win.hide();
  try {
    // Bring the main window forward, then ask main.js to open the Add
    // Torrent dialog with this magnet pre-filled. The dialog will fetch
    // metadata, show the file list, and let the user confirm — same flow
    // as if they'd pasted the magnet themselves.
    await invoke("focus_main");
    await emit("open-source", uri);
  } catch (e) {
    await emit("toast", { kind: "error", message: `Couldn't open main window: ${e}` });
  }
};
