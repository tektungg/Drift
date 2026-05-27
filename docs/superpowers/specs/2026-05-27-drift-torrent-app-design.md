# Drift — Design Spec

**Date:** 2026-05-27
**Status:** Approved for implementation planning
**Scope:** Single personal Windows desktop app for downloading torrents, with an intentionally Claude-inspired visual aesthetic.

---

## 1. Purpose

Drift is a personal, native Windows torrent client. The user has no desktop development experience and wants a clean, modern interface that mirrors Claude's visual language (warm cream palette, coral accent, serif/sans pairing, generous whitespace). The app is for the user's own machine only — no distribution, no signing, no auto-update infrastructure required.

The core value is the look and feel and a small set of quality-of-life features over existing free clients (qBittorrent, Deluge). Drift is not trying to outperform mature engines on raw speed; it inherits speed from its embedded library.

---

## 2. Stack

| Concern | Choice |
|---|---|
| Shell / window | Tauri 2 (Rust backend, webview frontend) |
| Torrent engine | librqbit (Rust, async, embedded in the Rust process) |
| Frontend | Plain HTML / CSS / JavaScript — no framework |
| Clipboard listener | `clipboard-win` crate (event-driven via Windows `AddClipboardFormatListener`) |
| Persistence | JSON files in `%APPDATA%\Drift\` |
| Distribution | Single `Drift.exe` installer (~10 MB) — installed locally, no codesigning |

**Why this stack:**

- Tauri produces a small native window but allows web-style CSS, which is required to faithfully reproduce Claude's typography and palette.
- librqbit handles DHT, PEX, uTP, magnet links, and multi-tracker logic. No protocol implementation work needed.
- A vanilla-JS frontend is justified by the small surface (three screens) and keeps the bundle minimal with no framework upgrade burden.

---

## 3. Scope (locked)

### In scope (v1)
- Add torrents via magnet link paste, `.torrent` file picker, or drag-and-drop anywhere on the main window.
- List of torrents with name, progress, speed, ETA, and state (Downloading / Seeding / Completed / Paused).
- Pause, resume, remove, and remove-with-files actions per torrent.
- Per-torrent file selection (deselect unwanted files in a torrent at Add time or mid-download).
- Auto-categorization of completed files into subfolders by file type (Video, Audio, Documents, Compressed, Programs, Images, Other).
- System tray icon with Show/Hide, Pause All, Quit. Closing the main window hides to tray.
- Global speed limits (down/up) in settings.
- Single-instance enforcement — relaunching focuses the existing window.
- Clipboard watcher: when a magnet link is copied, a small tray-anchored toast offers to add it.

### Out of scope (deferred or excluded)
- Sequential download / streaming
- Built-in search
- RSS feeds
- Browser magnet-link protocol handler registration
- Categories beyond file-type routing (no user labels in v1)
- Dark mode
- Multi-OS support (Windows only)
- Auto-update channels

---

## 4. Architecture

### Processes

A single OS process. The Rust binary owns:
- the librqbit session,
- application state and persistence,
- the clipboard listener thread,
- the system tray integration,
- the Tauri webview that renders the UI.

The webview is purely a renderer — all state lives in Rust. The frontend talks to Rust via Tauri's `invoke` command bridge.

**Single-instance enforcement** uses the `tauri-plugin-single-instance` plugin. A second launch sends its argv to the first instance over a named pipe; the first instance focuses its main window and, if argv contains a magnet link or `.torrent` path, opens the Add dialog pre-filled.

### Windows

- **Main window** — the sidebar + list UI. Custom titlebar matching the cream background.
- **Magnet toast window** — borderless, ~360×120 px, bottom-right above the tray. Does not steal focus. Created on-demand by the clipboard listener.
- **Add Torrent dialog** — modal overlay rendered inside the main window (not a separate OS window).
- **Settings panel** — slide-in panel from the right inside the main window.

### Persisted state

```
%APPDATA%\Drift\
  state.json       — list of torrents (infohash, display name, save path, label, paused/active, added timestamp)
  config.json      — global settings (default folder, category map, speed limits, clipboard toggle, start-with-Windows toggle, close-to-tray toggle)
  resume\          — librqbit's session resume files (one per torrent)
  logs\            — rotating log files (debug aid only; not surfaced in UI in v1)
```

State writes happen on every meaningful change (add, remove, pause, resume, settings change). Crash recovery: on launch, load `state.json`, hand each torrent's resume file back to librqbit, restore paused/active state.

---

## 5. UI design

### Visual language

- **Palette:** background `#F5F1E8` (cream), surface `#FAF7F0`, ink `#1F1E1D`, ink-soft `#6B645A`, line `#E8E1D2`, accent `#D97757` (coral), accent-soft `#F4E0D4`.
- **Typography:** serif headings (system fallback chain: `ui-serif, "Charter", "Iowan Old Style", Georgia, serif`), sans body (`ui-sans-serif, "Inter", system-ui, sans-serif`). No bundled proprietary fonts.
- **Density:** generous — 14–22 px gutters, 8–10 px row padding, soft 1 px borders, no harsh shadows.
- **Theme:** light only in v1.

### Main view (layout: sidebar + list)

**Sidebar (left, ~160 px):**
- Filters: All · Downloading · Seeding · Completed · Paused, each with a live count.
- Bottom: aggregate down/up speeds.
- Selected filter highlighted with `accent-soft` background.

**Main list (right):**
- Top row: filter title (e.g., "All downloads"), "+ Add" pill button (coral).
- Each torrent row: name, size progress line ("2.4 / 3.9 GB · 12.4 MB/s"), thin progress bar in coral, right-aligned state label + percent.
- **Click a row** → expands inline to show: file list (with per-file progress and a checkbox to deselect mid-download), peer count, ratio, added/ETA timestamps.
- **Right-click a row** → context menu: Pause / Resume · Open Folder · Copy Magnet · Remove · Remove + Delete Files.

### Add Torrent dialog

Modal overlay, ~520 px wide:
- Large paste field: *"Paste a magnet link or drop a `.torrent` file"*.
- Drag-and-drop accepted on the field and on the main window background.
- Once parsed: shows torrent name, total size, file tree with checkboxes, and an auto-categorization preview line (e.g., *"Will go to `Downloads\Drift\Video\` (largest file is `.mkv`)"*).
- Optional **Override folder** dropdown lets the user force a different category or pick any path.
- Buttons: **Add** (coral) · **Cancel**.

### Settings panel

Slide-in from the right:
- Default download root (path picker, default `%USERPROFILE%\Downloads\Drift\`)
- Editable category-to-extensions map (defaults pre-filled per section 6)
- Global speed limits (down/up in KB/s, 0 = unlimited)
- Clipboard watcher toggle (default on)
- Start with Windows toggle
- Close-button-keeps-app-in-tray toggle (default on)

### Magnet toast

Triggered by the clipboard listener (see section 7):
- Small borderless window, bottom-right above the tray.
- Shows the torrent's display name (from the magnet's `dn=` parameter, or the truncated infohash if missing).
- Two buttons: **Add** (coral) · **Dismiss**.
- Auto-dismisses after 10 seconds.
- Does not steal focus.

---

## 6. Auto-categorization

### Principle

Categorization happens **at Add time**, not on completion. Once librqbit has the torrent's metadata (file list), Drift computes the category, sets the download folder once, and bytes write directly to the final location. No post-completion move or copy.

### Folder structure

```
%USERPROFILE%\Downloads\Drift\
  Video\
  Audio\
  Documents\
  Compressed\
  Programs\
  Images\
  Other\
```

### Default category-to-extension map

| Category | Extensions |
|---|---|
| Video | mp4, mkv, avi, mov, wmv, flv, webm, m4v, mpg, mpeg, ts, m2ts |
| Audio | mp3, flac, wav, aac, ogg, m4a, wma, opus, alac |
| Documents | pdf, epub, mobi, doc, docx, xls, xlsx, ppt, pptx, txt, rtf, csv |
| Compressed | zip, rar, 7z, tar, gz, bz2, xz |
| Programs | exe, msi, dmg, deb, rpm, apk, appimage, iso, img |
| Images | jpg, jpeg, png, gif, webp, bmp, svg, tiff, raw, heic |
| Other | *(fallback for anything unmatched)* |

Editable in Settings; the map is stored in `config.json`. Editing the map only affects future Add operations — existing torrents keep their original save path.

### Routing rule

- **Single-file torrents** → routed by that file's extension. `ubuntu-24.04.iso` → `Drift\Programs\ubuntu-24.04.iso`.
- **Multi-file torrents** → routed by the **largest file's** extension. The whole torrent folder lands intact inside that category. A movie release (`.mkv` + `.srt` + `.nfo`) → `Drift\Video\<release-folder>\`. A music album (FLACs + `cover.jpg`) → `Drift\Audio\<album-folder>\`.
- **Override** in the Add dialog wins. Override path is persisted per-torrent so resumes never re-categorize.
- **Unknown extension** → `Other\`.
- **Tiebreak** when two file extensions sum to identical byte totals: prefer the category with the higher-priority extension in this order: Video → Audio → Programs → Compressed → Documents → Images → Other. (Vanishingly rare; this just makes the behavior deterministic.)

### Folder collisions

If the destination already exists:
- If the existing path has a matching infohash in `state.json`, librqbit resumes it.
- Otherwise, suffix the destination folder name with ` (2)`, ` (3)`, etc. to avoid overwriting unrelated files.

---

## 7. Clipboard watcher

### Mechanism

- Registered via Windows `AddClipboardFormatListener` (event-driven, zero CPU when idle).
- Lives in a dedicated thread inside the Rust process, started on app launch, stopped on quit.

### Behavior

- On clipboard change, attempt to read text. If text starts with `magnet:?xt=urn:btih:`, parse it.
- If the infohash is already in `state.json`, do nothing.
- If the infohash was dismissed earlier in the current session, do nothing.
- Otherwise, spawn the magnet toast window (section 5).

### Settings

- Toggle in Settings, default **on**.
- When toggled off, the listener thread is paused but not torn down.

### Non-goals

- Watcher never logs clipboard contents.
- Watcher never inspects non-`magnet:` strings beyond the prefix check.

---

## 8. Data flow — Add → Download → Complete

1. User pastes a magnet link or drops a `.torrent` file into the Add dialog (or accepts the magnet toast).
2. Frontend calls Rust `add_torrent(source)`.
3. Rust hands source to librqbit.
   - For a `.torrent` file, metadata is available immediately.
   - For a magnet link, librqbit must fetch metadata from peers first. The Add dialog shows a *"Fetching metadata…"* state until it arrives (typically 1–10 seconds). If metadata fails to arrive within 60 seconds, the dialog shows a retry button.
   - librqbit returns metadata (name, file list, total size, infohash).
4. Dedup check: if infohash already in `state.json`, abort with `AlreadyExists` and the frontend scrolls the existing row into view.
5. Rust computes the category from the file list using the rules in section 6 and constructs the destination path.
6. Rust calls librqbit `start_torrent(infohash, save_path, selected_files)`.
7. Rust appends an entry to `state.json` and writes it to disk.
8. librqbit emits progress events at ~1 Hz. Rust forwards them to the frontend via Tauri events. The frontend updates the row in place.
9. On completion, the row's state changes to **Seeding** (the torrent stays in the session until removed). Files are already at their final path — no move step.

---

## 9. Error handling

| Situation | Behavior |
|---|---|
| Invalid magnet or corrupt `.torrent` | Toast: *"Couldn't read this torrent"*. No state change. |
| Disk full or write permission denied | Pause torrent, toast with the failing path. User can fix and resume. |
| Network drop / DNS hiccup | librqbit auto-reconnects. If no progress for 30 s, row shows **Stalled**. |
| Listen port already in use | librqbit picks another free port automatically. Settings shows the active port. |
| Duplicate add (same infohash) | Scroll the existing row into view; toast *"Already in your list."* |
| Crash / forced quit / power loss | On next launch, `state.json` + librqbit resume files restore prior state. No full re-hash if files are intact. |
| Clipboard watcher sees non-magnet text | Silently ignored. |

---

## 10. Testing approach

Personal-use app — testing is scaled accordingly.

**Rust unit tests:**
- Category resolver: single-file, multi-file largest-wins, override honored, unknown extension → Other.
- Magnet parser and infohash dedup.
- `state.json` round-trip serialization.

**One Rust integration test:**
- Add a known local test torrent end-to-end: metadata fetch → category resolution → librqbit start → completion → file lands at expected path → pause → resume → restart engine → resumes from prior state.

**Manual smoke checklist** (`docs/smoke-checklist.md`):
- Add via paste, file picker, drag-drop, and magnet toast.
- Sidebar filters update counts.
- Tray Show/Hide, Pause All, Quit.
- Settings round-trip (change default folder, change category map, toggle clipboard watcher).
- Magnet toast: appears on copy, dedups, dismisses on click-out, respects setting toggle.

**Not tested:**
- librqbit internals (upstream-tested).
- Tauri window and tray APIs (vendor-tested).

---

## 11. Open questions / risks

- **Custom titlebar on Windows 11** — Tauri's decorations API needs a small dance to keep snap-layouts working. Implementation will follow the documented pattern; risk is low but worth verifying early in the build.
- **Magnet toast positioning** — multi-monitor setups need testing; default behavior is "screen containing the tray icon."
- **clipboard-win event coalescing** — rapid copy events may fire multiple times; the dedup-by-infohash check in the listener guards against double-prompting.

---

## 12. Out-of-scope follow-ups (for a hypothetical v1.1)

- Sequential download / streaming
- Browser magnet protocol handler registration (`magnet:` URLs open Drift)
- User-defined labels and label-routing rules
- RSS feeds
- Dark mode (palette already designed around a warm-charcoal counterpart)
