# Drift — Help & About in Settings

**Status:** Design approved (with visual mockups), ready for planning
**Date:** 2026-05-29
**Author:** brainstormed with Claude

## Goal

Give Drift a built-in **Help** guide and an **About** screen, both inside the
existing Settings drawer, accessed via a tab switcher at the top of the panel.
Help walks the user through the app's features (beyond the existing empty-state
"how to add" hints); About shows identity, version, and links.

Frontend-mostly, plus two tiny Tauri commands (version + open-url). Targets a
**0.5.0** minor release (new feature).

**In scope:** a `Settings · Help · About` tab switcher in the panel; a Help view
(collapsible Q&A + GitHub links); an About view (icon, name, version, tagline,
links, credits); supporting commands.
**Out of scope:** interactive guided tours/coachmarks; in-app changelog;
auto-update mechanism (the "check for updates" link just opens the Releases page).

---

## Panel structure — tab switcher

The Settings drawer (`#settings-panel`, opened by `toggleSettings()`) gains a
segmented switcher at the very top, reusing the existing `.seg`/`.seg-btn`
visual language (already used by the theme toggle), with three tabs:
**Settings · Help · About**.

- A module-level `settingsTab` variable (`"settings" | "help" | "about"`,
  default `"settings"`) tracks the active view. It resets to `"settings"` each
  time the panel is opened fresh (so opening Settings always lands on Settings).
- `toggleSettings()` builds the switcher + the active tab's body. Clicking a tab
  updates `settingsTab` and re-renders the panel body (not the whole drawer
  open/close).
- The existing **Save / Close** footer is shown ONLY on the Settings tab. Help
  and About are read-only, so they show just a **Close** button.
- The current Settings content (Downloads, Appearance, Behavior, Queue, Category
  extensions) moves unchanged into a `renderSettingsBody()` function; its save
  wiring is unchanged.

Decomposition: split `toggleSettings()`'s giant template into three body
builders — `renderSettingsBody()`, `renderHelpBody()`, `renderAboutBody()` —
each returning an HTML string, plus a small `renderSettingsPanel()` that emits
the switcher + the active body + footer and rewires events. This keeps each
view focused and the file readable.

---

## Help view

Structure:
1. A one-line intro: "Quick answers to get the most out of Drift."
2. A list of **collapsible entries** (`<details class="help-item">`, reusing the
   existing collapsible pattern from "Category extensions") — summary = question,
   body = answer. The first entry ("Adding torrents") is `open` by default; the
   rest are collapsed. A chevron icon on the right rotates when open.
3. A links row at the bottom with two buttons: **Full guide on GitHub** and
   **Report an issue** (open external URLs).

Entries (question → answer):
- **Adding torrents** — "Paste a magnet link, drag a `.torrent` file onto the
  window, click **+ Add torrent** to browse, or just copy a magnet — Drift
  watches your clipboard and offers to add it."
- **Where downloads go** — "Files are auto-sorted into category folders (Video,
  Audio, Documents, …). Folder-style torrents go to **Other/<name>**. Change the
  destination per-torrent in the Add dialog, or set the default folder in
  Settings."
- **Picking which files** — "Uncheck files you don't want in the Add dialog, or
  expand a row to change the selection even mid-download."
- **The download queue** — "Set **Max active downloads** in Settings; extras wait
  as **Queued** and start automatically as slots free up. Right-click a torrent
  to **Force start** (bypass the cap) or reorder its priority."
- **Selecting several at once** — "Ctrl-click to pick multiple torrents,
  Shift-click for a range, then use the action bar to pause, resume or remove
  them together."
- **Magnet links from your browser** — "Turn on **Open magnet links with Drift**
  in Settings, then clicking a magnet anywhere opens Drift with the Add dialog
  ready."
- **Seeding & opening files** — "Completed files stay shared with other peers —
  and you can open or run them **while Drift keeps seeding**."
- **Closing to the tray** — "Closing the window keeps Drift seeding in the system
  tray. Click the tray icon to bring it back."

Answers may contain simple `<b>` emphasis; all dynamic text is static here (no
user input), so no escaping concerns.

---

## About view

Centered layout:
- The **wave** app icon (existing `wave` glyph in `icons.js`) at ~64px, accent color.
- **Drift** (serif heading).
- **Version <x.y.z>** — fetched at panel-open from the new `app_version` command.
- Tagline: "A clean, fast, native Windows torrent client with a warm,
  Claude-inspired interface."
- A vertical stack of link buttons:
  - **GitHub repository** → `https://github.com/tektungg/Drift`
  - **Releases · check for updates** → `https://github.com/tektungg/Drift/releases`
  - **License (MIT)** → `https://github.com/tektungg/Drift/blob/main/LICENSE`
- Credits footer: "Built with Tauri 2 · Rust · librqbit" and "Not affiliated with
  Anthropic — just a fan of the aesthetic."

---

## Plumbing (Rust)

Two new Tauri commands in `commands.rs`, registered in `main.rs`'s
`generate_handler!`:

```rust
/// App version (from Cargo). Shown on the About screen.
#[tauri::command]
pub fn app_version() -> String { env!("CARGO_PKG_VERSION").to_string() }

/// Open an external URL in the user's default browser via tauri-plugin-opener.
#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
```

(`tauri-plugin-opener` is already a dependency and registered. The exact opener
API call is verified against the installed plugin version during implementation;
if the signature differs, adapt minimally — the contract is "open this https URL
in the default browser".)

**Safety:** `open_url` is only ever called by the frontend with the three
hard-coded About links and the two hard-coded Help links — no user-supplied URLs.

---

## Files touched

- `src/main.js` — `settingsTab` state; split `toggleSettings()` into
  `renderSettingsPanel()` + `renderSettingsBody()` / `renderHelpBody()` /
  `renderAboutBody()`; tab-switch wiring; fetch `app_version`; link clicks →
  `invoke("open_url", { url })`.
- `src/styles.css` — `.settings-tabs`/`.tab`, `.help-item` (summary, chevron,
  body), `.help-links`, `.about` (glyph, heading, version, tagline),
  `.about-links`, credits.
- `src/icons.js` — reuse `wave`, `link`, `chevron` (all already present after the
  controls-refinement work). Add `github` icon only if a distinct mark is wanted;
  otherwise reuse `link`. (Decision: reuse `link` — no new icon needed.)
- `src-tauri/src/commands.rs` — `app_version`, `open_url`.
- `src-tauri/src/main.rs` — register the two commands.
- `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml` — version bump to 0.5.0.

No `state.json`/`config.json` changes.

---

## Testing

- **Unit (Rust):** `app_version()` returns a non-empty string equal to
  `env!("CARGO_PKG_VERSION")` (trivial test in `commands.rs` or an integration
  test). `open_url` is thin glue over the plugin — covered by manual smoke, not
  unit-tested.
- **Parse:** `node --check src/main.js`.
- **Manual smoke:** open Settings → tabs show; Settings tab unchanged
  (save/close still works); Help tab shows entries, first open, others toggle,
  chevron rotates; both Help links open the browser; About shows the real
  version (matches the release), tagline, and all three links open the correct
  pages; switching tabs keeps the drawer open; reopening Settings lands on the
  Settings tab; light + dark both look right.

## Risks

- **Opener API signature** may differ slightly across `tauri-plugin-opener`
  versions — verify the exact `open_url`/`open` call during implementation
  (low risk; single call site).
- **Footer button context:** the Save button must not appear on Help/About (they
  have no form). Handled by `renderSettingsPanel()` emitting the footer
  per-tab.
- **`toggleSettings()` refactor regressions:** moving the existing settings
  markup into `renderSettingsBody()` must preserve every id and the save handler
  wiring exactly. Mitigated by moving the markup verbatim and keeping the same
  element ids.
