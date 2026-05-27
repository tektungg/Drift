import { invoke } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/core.js";
import { listen } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/event.js";
import { getCurrentWindow } from "https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/window.js";

const win = getCurrentWindow();
let pending = null;

listen("magnet-detected", async (e) => {
  pending = e.payload;
  document.getElementById("t-name").textContent = pending.name;
  await win.show();
  // 10s auto-dismiss
  setTimeout(() => { if (pending) { win.hide(); pending = null; } }, 10000);
});

document.getElementById("t-dismiss").onclick = async () => { pending = null; await win.hide(); };
document.getElementById("t-add").onclick = async () => {
  if (!pending) return;
  try {
    await invoke("add_torrent", { req: { source: pending.uri, overridePath: null, selectedFiles: null } });
  } catch (e) {
    // failures are surfaced in the main window's toast stack via listen("toast")
  }
  pending = null;
  await win.hide();
};
