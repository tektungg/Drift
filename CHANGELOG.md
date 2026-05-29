# Changelog

All notable changes to Drift are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
aims to follow semantic-ish versioning.

## [Unreleased]

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
