# Drift smoke checklist

Run before every release (and after major changes). All on Windows 11.

## Cold start
- [ ] Launching `Drift.exe` opens main window in <2s
- [ ] Cream titlebar, sidebar visible with 5 filters at 0 counts (first run)

## Add flow
- [ ] Click "+ Add torrent" → modal opens
- [ ] Paste a known-good magnet → metadata fetches within 10s, name + total size show
- [ ] File tree appears with all files checked
- [ ] Predicted save path shows the correct category (Video for a .mkv release, etc.)
- [ ] Click Add → modal closes, row appears in list, downloading starts

## Drag-drop
- [ ] Drag a .torrent file from Explorer onto window → Add dialog opens pre-filled

## List interactions
- [ ] Click a row → expands, shows file list with per-file %
- [ ] Right-click → menu shows Pause/Open Folder/Copy Magnet/Remove
- [ ] Pause action → row state changes to "paused", progress halts
- [ ] Resume → progress resumes
- [ ] Open Folder → File Explorer opens correct path
- [ ] Copy Magnet → clipboard contains a valid magnet URI

## Sidebar
- [ ] Counts update live as state changes
- [ ] Clicking each filter narrows the list
- [ ] Bottom shows aggregate down/up speeds

## Settings
- [ ] Opens via header button, slides in from right
- [ ] Change default folder, save, reopen → value persists
- [ ] Change speed limits → live torrents respect new limits
- [ ] Toggle clipboard watcher off → magnet copy no longer triggers toast
- [ ] Edit category map (e.g. add ".part" to compressed) → save persists

## Magnet toast
- [ ] Copy a magnet not in list → toast appears bottom-right
- [ ] Click Add → torrent added via normal flow
- [ ] Dismiss → toast hides
- [ ] Copy same magnet again → no toast (deduped this session)
- [ ] Auto-dismiss after 10s if untouched

## Tray
- [ ] Tray icon visible
- [ ] Left-click toggles main window visibility
- [ ] Right-click → menu Show/Pause All/Quit
- [ ] Close window → app stays in tray (downloads continue)
- [ ] Pause All → all torrents pause

## Single-instance + argv
- [ ] Launch Drift, then run `drift.exe "magnet:?xt=..."` from another shell
- [ ] First instance focuses, Add dialog opens pre-filled, no second window spawns

## Persistence
- [ ] Quit Drift mid-download (via tray)
- [ ] Relaunch → torrent appears immediately, progress continues without rehash

## Theming sanity
- [ ] Coral accent used on primary actions, progress bars
- [ ] Serif used on headings ("Drift", "All downloads", "Add torrent")
- [ ] No system-default chrome visible (custom titlebar in use)

## UI polish (0.2.0)
- [ ] Each state shows the correct dot color + tinted progress (downloading=coral, seeding=sage, completed=teal, paused=gray, stalled=amber)
- [ ] Torrent rows show a file-type line icon (or folder icon for folder torrents)
- [ ] ETA shows a sensible value while downloading, "—" when paused/stalled/0 speed
- [ ] Sidebar filters have icons + live counts; totals card shows aggregate down/up
- [ ] Empty "All" filter shows the wave glyph + three hint cards; other empty filters show the lighter variant
- [ ] Settings is grouped (Downloads / Behavior / Categories); the three booleans are toggle switches that flip coral/gray and persist after Save + reopen
- [ ] Add dialog file rows show per-file line icons; long save paths still front-truncate
- [ ] Row hover highlights; progress bars animate smoothly on the 1 Hz updates with no flicker
