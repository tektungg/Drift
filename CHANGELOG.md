# Changelog

All notable changes to Drift are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
aims to follow semantic-ish versioning.

## [Unreleased]

## [0.5.1] — 2026-06-03
### Added
- **"Check for updates" button** in About for an on-demand update check, with a
  live download **progress toast** while an update installs.
### Changed
- The auto-updater no longer nags: if you dismiss an available update with
  "Later", it stays quiet for the rest of the session (the manual button still
  works).
- Installer polish: branded NSIS header/sidebar images, and uninstall now offers
  to remove Drift's settings/torrent list (downloads are never touched). Pinned a
  minimum WebView2 version.

## [0.5.0] — 2026-06-02
### Added
- **System magnet/`.torrent` association**: the installer now registers Drift as
  the handler for `magnet:` links and `.torrent` files, so clicking a magnet in
  the browser or double-clicking a `.torrent` in Explorer opens it in Drift
  (via `tauri-plugin-deep-link` + a `fileAssociations` entry).
- **In-app auto-updater** (`tauri-plugin-updater`): on launch Drift checks the
  GitHub Releases `latest.json` for a newer **signed** build and offers to
  download and install it.

  > Note: builds up to and including 0.4.1 shipped without the updater, so
  > existing users must download 0.5.0 manually once; auto-updates work from
  > 0.5.0 onward.
### Changed
- Installer now lets you choose a **per-user or all-users** install (NSIS
  `installMode: both`) instead of always elevating, and shows a language
  selector. Added an installer icon and the EULA (LICENSE) page.
- Synced `package.json` version with `tauri.conf.json`.

## [0.4.0] — 2026-05-29
### Added
- **Search** the download list by name, plus a custom **sort** menu
  (date added / name / progress / speed / size) with a direction toggle.
- **Multi-select** with Ctrl/Shift-click and a bulk action bar (pause / resume / remove).
- **Download queue**: a *Max active downloads* cap; extra torrents wait as
  **Queued** and start automatically as slots free up. Right-click to
  **Force start** (bypass the cap) or reorder priority.
### Changed
- Vendored a lightly-patched librqbit so **completed files stay openable/runnable
  while the torrent keeps seeding** (Windows previously held a write lock).
### Fixed
- Magnet links now open reliably when Drift is already running.
- Saving settings no longer hangs or flashes console windows.
- Eliminated the 1 Hz list flicker; row hover scoped to the header; added a
  minimal custom scrollbar.

## [0.3.0] — 2026-05-28
### Added
- **Dark mode** with a System / Light / Dark theme toggle in Settings.
- **Magnet-link handler**: opt-in setting to open `magnet:` links from the
  browser directly in Drift.
- Browse for a `.torrent` file from the Add dialog; expanded-row details
  (peers, ratio, uploaded, date added, ETA) and live per-file progress.

## [0.2.2] — 2026-05-28
### Fixed
- System tray fixes (single icon; correct left-click behavior).

## [0.2.1] — 2026-05-28
### Added
- New wave app icon.

## [0.2.0] — 2026-05-28
### Changed
- UI/UX polish pass: refined torrent rows, sidebar, empty state, settings
  sections, and file-type icons.

[Unreleased]: https://github.com/tektungg/Drift/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/tektungg/Drift/releases/tag/v0.4.0
[0.3.0]: https://github.com/tektungg/Drift/releases/tag/v0.3.0
[0.2.2]: https://github.com/tektungg/Drift/releases/tag/v0.2.2
[0.2.1]: https://github.com/tektungg/Drift/releases/tag/v0.2.1
[0.2.0]: https://github.com/tektungg/Drift/releases/tag/v0.2.0
