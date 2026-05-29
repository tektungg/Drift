# Help & About Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `Settings · Help · About` tab switcher to the Settings drawer, with a collapsible Help guide (+ GitHub links) and an About view (app icon, version, tagline, repo/releases/license links).

**Architecture:** Two tiny Tauri commands (`app_version`, `open_url`) back the About data + external links. The frontend splits the monolithic `toggleSettings()` into a panel shell (`renderSettingsPanel`) + three body builders (`renderSettingsBody`/`renderHelpBody`/`renderAboutBody`), tracked by a `settingsTab` state var. The existing settings markup + save wiring move verbatim into `renderSettingsBody`. Styling reuses the existing segmented control and `<details>` patterns.

**Tech Stack:** Vanilla HTML/CSS/JS (ES modules), Tauri 2, `tauri-plugin-opener` (already a dep; `opener:default` capability already granted). No Rust crates added.

**Spec:** `docs/superpowers/specs/2026-05-29-help-and-about-design.md`

---

## File Structure

**Modified:**
- `src-tauri/src/commands.rs` — add `app_version()` + `open_url(url)` commands (+ a unit test for `app_version`).
- `src-tauri/src/main.rs` — register the two commands in `generate_handler!`.
- `src/main.js` — `settingsTab` state; refactor `toggleSettings()` into shell + 3 body builders; tab wiring; About fetches version; link clicks call `open_url`.
- `src/styles.css` — `.settings-tabs`/`.tab`, `.help-item` (summary/chevron/body), `.help-links`, `.about` + `.about-links` + credits.
- `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` — version → 0.4.1.

Reused icons (already in `icons.js`): `wave`, `link`, `chevron`. No new icons.

---

### Task 1: Backend commands — `app_version` + `open_url`

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write a failing test for `app_version`**

In `src-tauri/src/commands.rs`, add a test module at the end of the file (if a `#[cfg(test)] mod tests` already exists in this file, add the test inside it instead):

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn app_version_matches_cargo() {
        assert_eq!(super::app_version(), env!("CARGO_PKG_VERSION"));
        assert!(!super::app_version().is_empty());
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `export PATH="/c/Users/ramap/.cargo/bin:$PATH" && cd "D:/Personal Project/Drift/src-tauri" && cargo test --lib commands::tests::app_version_matches_cargo`
Expected: FAIL to compile — `app_version` not found.

- [ ] **Step 3: Add the two commands**

In `src-tauri/src/commands.rs`, add (e.g. near the other small commands like `open_folder`):

```rust
/// The app version (from Cargo). Shown on the About screen.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Open an external URL in the user's default browser. Called only with the
/// hard-coded About/Help links from the frontend — never user input.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}
```

(Mirrors the existing `open_folder`, which uses `tauri_plugin_opener::open_path`.)

- [ ] **Step 4: Register both commands**

In `src-tauri/src/main.rs`, find the `tauri::generate_handler![ ... ]` list and add these two lines (e.g. after `commands::move_in_queue,`):

```rust
            commands::app_version,
            commands::open_url,
```

- [ ] **Step 5: Build + test**

Run: `export PATH="/c/Users/ramap/.cargo/bin:$PATH" && cd "D:/Personal Project/Drift/src-tauri" && cargo build && cargo test --lib`
Expected: compiles; all lib tests pass (including the new `app_version_matches_cargo`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "Add app_version and open_url commands"
```

---

### Task 2: Styles for tabs, Help entries, and About

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Append the new styles**

Append to `src/styles.css`:

```css
/* Settings drawer tab switcher (Settings / Help / About). */
.settings-tabs { display: flex; gap: 3px; background: var(--surface); border: 1px solid var(--line);
  border-radius: 9px; padding: 3px; margin-bottom: 18px; }
.settings-tabs .tab { flex: 1; text-align: center; padding: 7px 0; border-radius: 6px; font-size: 13px;
  color: var(--ink-soft); cursor: pointer; border: 0; background: transparent; font-family: var(--font-sans); }
.settings-tabs .tab:hover { color: var(--ink); }
.settings-tabs .tab.active { background: var(--accent-soft); color: var(--accent); font-weight: 500; }

/* Help view */
.help-intro { font-size: 13px; color: var(--ink-soft); margin-bottom: 14px; line-height: 1.5; }
.help-item { border-bottom: 1px solid var(--line); }
.help-item summary { cursor: pointer; padding: 11px 2px; font-size: 13px; color: var(--ink);
  list-style: none; display: flex; align-items: center; gap: 8px; }
.help-item summary::-webkit-details-marker { display: none; }
.help-item summary .arno { margin-left: auto; color: var(--ink-soft); transition: transform .15s ease;
  width: 15px; height: 15px; display: inline-flex; }
.help-item summary .arno svg { width: 15px; height: 15px; }
.help-item[open] summary .arno { transform: rotate(90deg); }
.help-item .body { font-size: 12.5px; color: var(--ink-soft); line-height: 1.6; padding: 2px 2px 12px; }
.help-links { display: flex; gap: 8px; margin-top: 18px; }
.help-links .lk { flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px;
  text-decoration: none; color: var(--ink); border: 1px solid var(--line); background: var(--surface);
  border-radius: 8px; padding: 9px; font-size: 13px; cursor: pointer; }
.help-links .lk:hover { border-color: var(--accent); }
.help-links .lk svg { width: 15px; height: 15px; color: var(--ink-soft); }

/* About view */
.about { text-align: center; padding: 12px 0 4px; }
.about .glyph { width: 64px; height: 64px; margin: 8px auto 14px; color: var(--accent); display: inline-flex; }
.about .glyph svg { width: 64px; height: 64px; }
.about h3 { font-family: var(--font-serif); font-weight: 500; font-size: 22px; margin: 0 0 2px; }
.about .ver { font-size: 12px; color: var(--ink-soft); margin-bottom: 12px; }
.about .tag { font-size: 13px; color: var(--ink); margin: 0 auto 20px; max-width: 280px; line-height: 1.5; }
.about-links { display: flex; flex-direction: column; gap: 8px; margin-bottom: 22px; }
.about-links .lk { display: flex; align-items: center; justify-content: center; gap: 8px;
  text-decoration: none; color: var(--ink); border: 1px solid var(--line); background: var(--surface);
  border-radius: 8px; padding: 9px; font-size: 13px; cursor: pointer; }
.about-links .lk:hover { border-color: var(--accent); }
.about-links .lk svg { width: 15px; height: 15px; color: var(--ink-soft); }
.about .credits { font-size: 11px; color: var(--ink-soft); line-height: 1.6; }
```

- [ ] **Step 2: Commit**

```bash
git add src/styles.css
git commit -m "Style settings tabs, Help entries, and About view"
```

---

### Task 3: Refactor `toggleSettings` into shell + Settings body (no behavior change)

**Files:**
- Modify: `src/main.js`

This task ONLY restructures the existing Settings panel — Help/About come in Task 4. The Settings tab must behave EXACTLY as today after this task.

- [ ] **Step 1: Add the tab state variable**

In `src/main.js`, near the other module-level state (e.g. after `let sortDir = ...`), add:

```js
let settingsTab = "settings"; // "settings" | "help" | "about"
```

- [ ] **Step 2: Replace `toggleSettings()` with a shell + Settings body**

Replace the ENTIRE current `toggleSettings()` function (the whole function from `async function toggleSettings() {` through its closing `}` at the `s-save` handler) with the following. This moves the existing markup verbatim into `renderSettingsBody(cfg)` and the existing wiring into `wireSettingsBody(panel)`, and adds the shell:

```js
async function toggleSettings() {
  const panel = document.getElementById("settings-panel");
  if (panel.classList.contains("open")) { panel.classList.remove("open"); return; }
  settingsTab = "settings";          // always open on the Settings tab
  await renderSettingsPanel();
  panel.classList.add("open");
}

async function renderSettingsPanel() {
  const panel = document.getElementById("settings-panel");
  const tabs = `
    <div class="settings-tabs" id="settings-tabs">
      <button class="tab ${settingsTab === "settings" ? "active" : ""}" data-tab="settings">Settings</button>
      <button class="tab ${settingsTab === "help" ? "active" : ""}" data-tab="help">Help</button>
      <button class="tab ${settingsTab === "about" ? "active" : ""}" data-tab="about">About</button>
    </div>`;

  let body, footer;
  if (settingsTab === "settings") {
    const cfg = await invoke("get_settings");
    body = renderSettingsBody(cfg);
    footer = `<div style="display:flex; justify-content:flex-end; gap:8px; margin-top:18px">
        <button class="btn-ghost" id="s-cancel">Close</button>
        <button class="btn-primary" id="s-save">Save</button>
      </div>`;
  } else if (settingsTab === "help") {
    body = renderHelpBody();
    footer = `<div style="display:flex; justify-content:flex-end; margin-top:18px">
        <button class="btn-ghost" id="s-cancel">Close</button></div>`;
  } else {
    body = await renderAboutBody();
    footer = `<div style="display:flex; justify-content:flex-end; margin-top:18px">
        <button class="btn-ghost" id="s-cancel">Close</button></div>`;
  }

  panel.innerHTML = tabs + body + footer;

  // Tab switching.
  panel.querySelectorAll("#settings-tabs .tab").forEach(b => b.onclick = () => {
    settingsTab = b.dataset.tab;
    renderSettingsPanel();
  });
  // Close button (present on every tab).
  const cancel = document.getElementById("s-cancel");
  if (cancel) cancel.onclick = () => panel.classList.remove("open");

  if (settingsTab === "settings") wireSettingsBody(panel);
  else if (settingsTab === "help") wireHelpBody(panel);
  else wireAboutBody(panel);
}

function renderSettingsBody(cfg) {
  return `
    <div class="settings-group">
      <div class="group-label">Downloads</div>
      <div class="settings-row"><span>Default download folder</span></div>
      <input type="text" id="s-root" value="${escape(cfg.download_root)}" style="margin-bottom:10px">
      <div class="settings-row"><span>Download limit (KB/s)</span>
        <input class="num" type="text" id="s-down" value="${cfg.download_kbps}"></div>
      <div class="settings-row"><span>Upload limit (KB/s)</span>
        <input class="num" type="text" id="s-up" value="${cfg.upload_kbps}"></div>
    </div>

    <div class="settings-group">
      <div class="group-label">Appearance</div>
      <div class="settings-row"><span>Theme</span>
        <div class="seg" id="s-theme">
          <button class="seg-btn ${(cfg.theme || 'system') === 'system' ? 'active' : ''}" data-val="system" title="System">${icon('system')}</button>
          <button class="seg-btn ${cfg.theme === 'light' ? 'active' : ''}" data-val="light" title="Light">${icon('light')}</button>
          <button class="seg-btn ${cfg.theme === 'dark' ? 'active' : ''}" data-val="dark" title="Dark">${icon('dark')}</button>
        </div>
      </div>
    </div>

    <div class="settings-group">
      <div class="group-label">Behavior</div>
      <div class="settings-row"><span>Watch clipboard for magnet links</span>
        <button class="switch ${cfg.clipboard_watch ? "" : "off"}" id="s-clip" data-on="${cfg.clipboard_watch}"></button></div>
      <div class="settings-row"><span>Close button hides to tray</span>
        <button class="switch ${cfg.close_to_tray ? "" : "off"}" id="s-tray" data-on="${cfg.close_to_tray}"></button></div>
      <div class="settings-row"><span>Start with Windows</span>
        <button class="switch ${cfg.start_with_windows ? "" : "off"}" id="s-startup" data-on="${cfg.start_with_windows}"></button></div>
      <div class="settings-row"><span>Open magnet links with Drift</span>
        <button class="switch ${cfg.magnet_handler ? "" : "off"}" id="s-magnet" data-on="${!!cfg.magnet_handler}"></button></div>
    </div>

    <div class="settings-group">
      <div class="group-label">Queue</div>
      <div class="settings-row"><span>Max active downloads (0 = unlimited)</span>
        <input class="num" type="text" id="s-maxactive" value="${cfg.max_active_downloads ?? 3}"></div>
    </div>

    <details class="settings-group">
      <summary style="cursor:pointer; font-size:10px; text-transform:uppercase; letter-spacing:0.07em; color:var(--ink-soft)">Category extensions</summary>
      ${["video","audio","documents","compressed","programs","images"].map(k => `
        <div class="settings-row" style="flex-direction:column; align-items:stretch; gap:6px">
          <label style="font-size:12px; color:var(--ink-soft)">${k}</label>
          <input type="text" id="s-cat-${k}" value="${escape(cfg.category_map[k].join(' '))}">
        </div>`).join("")}
    </details>`;
}

function wireSettingsBody(panel) {
  // Toggle switches: clicking flips the data-on flag and the .off class.
  panel.querySelectorAll(".switch").forEach(sw => sw.onclick = () => {
    const on = sw.dataset.on !== "true";
    sw.dataset.on = String(on);
    sw.classList.toggle("off", !on);
  });

  // Theme segmented control: mark active + apply live for instant preview.
  const themeBtns = panel.querySelectorAll("#s-theme .seg-btn");
  themeBtns.forEach(b => b.onclick = () => {
    themeBtns.forEach(x => x.classList.remove("active"));
    b.classList.add("active");
    applyTheme(b.dataset.val);
  });

  document.getElementById("s-save").onclick = async () => {
    const isOn = id => document.getElementById(id).dataset.on === "true";
    const themeBtn = panel.querySelector("#s-theme .seg-btn.active");
    const value = {
      download_root: document.getElementById("s-root").value,
      download_kbps: +document.getElementById("s-down").value || 0,
      upload_kbps: +document.getElementById("s-up").value || 0,
      clipboard_watch: isOn("s-clip"),
      close_to_tray: isOn("s-tray"),
      start_with_windows: isOn("s-startup"),
      magnet_handler: isOn("s-magnet"),
      max_active_downloads: +document.getElementById("s-maxactive").value || 0,
      theme: themeBtn ? themeBtn.dataset.val : "system",
      category_map: Object.fromEntries(
        ["video","audio","documents","compressed","programs","images"].map(k =>
          [k, document.getElementById("s-cat-"+k).value.split(/\s+/).filter(Boolean)])),
    };
    try {
      await invoke("set_settings", { value });
      applyTheme(value.theme);  // commit the chosen theme + sync localStorage
      document.getElementById("settings-panel").classList.remove("open");
      showToast("info", "Settings saved.");
    } catch (e) { showToast("error", friendlyError(e)); }
  };
}
```

- [ ] **Step 3: Add temporary stubs for the Help/About builders (filled in Task 4)**

So the file parses and the Settings tab works in isolation, add these stubs near the new functions (they will be REPLACED in Task 4):

```js
function renderHelpBody() { return `<div class="help-intro">Help — coming in the next step.</div>`; }
function wireHelpBody() {}
async function renderAboutBody() { return `<div class="about"><h3>Drift</h3></div>`; }
function wireAboutBody() {}
```

- [ ] **Step 4: Verify parse**

Run: `node --check src/main.js`
Expected: exit 0.

- [ ] **Step 5: Manual smoke (Settings unchanged)**

Run: `export PATH="/c/Users/ramap/.cargo/bin:$PATH" && cd "D:/Personal Project/Drift/src-tauri" && cargo run`. Open Settings: the tabs appear (Settings active). Confirm every Settings control still works — toggle a switch, change theme (live preview), edit a field, click **Save** → toast "Settings saved." and the drawer closes. Reopen → lands on Settings. Click Help/About → stub content shows; Close works. Close the app.

- [ ] **Step 6: Commit**

```bash
git add src/main.js
git commit -m "Refactor settings panel into tabbed shell + body builders (Settings unchanged)"
```

---

### Task 4: Help + About bodies

**Files:**
- Modify: `src/main.js`

- [ ] **Step 1: Replace the Help/About stubs with real implementations**

In `src/main.js`, replace the four stub functions from Task 3 Step 3 with:

```js
const HELP_ITEMS = [
  ["Adding torrents",
   "Paste a magnet link, drag a <b>.torrent</b> file onto the window, click <b>+ Add torrent</b> to browse, or just copy a magnet — Drift watches your clipboard and offers to add it."],
  ["Where downloads go",
   "Files are auto-sorted into category folders (Video, Audio, Documents, …). Folder-style torrents go to <b>Other/&lt;name&gt;</b>. Change the destination per-torrent in the Add dialog, or set the default folder in Settings."],
  ["Picking which files",
   "Uncheck files you don't want in the Add dialog, or expand a row to change the selection even mid-download."],
  ["The download queue",
   "Set <b>Max active downloads</b> in Settings; extras wait as <b>Queued</b> and start automatically as slots free up. Right-click a torrent to <b>Force start</b> (bypass the cap) or reorder its priority."],
  ["Selecting several at once",
   "Ctrl-click to pick multiple torrents, Shift-click for a range, then use the action bar to pause, resume or remove them together."],
  ["Magnet links from your browser",
   "Turn on <b>Open magnet links with Drift</b> in Settings, then clicking a magnet anywhere opens Drift with the Add dialog ready."],
  ["Seeding & opening files",
   "Completed files stay shared with other peers — and you can open or run them <b>while Drift keeps seeding</b>."],
  ["Closing to the tray",
   "Closing the window keeps Drift seeding in the system tray. Click the tray icon to bring it back."],
];

const GH_REPO = "https://github.com/tektungg/Drift";
const GH_RELEASES = "https://github.com/tektungg/Drift/releases";
const GH_LICENSE = "https://github.com/tektungg/Drift/blob/main/LICENSE";
const GH_ISSUES = "https://github.com/tektungg/Drift/issues";

function renderHelpBody() {
  const items = HELP_ITEMS.map(([q, a], i) => `
    <details class="help-item" ${i === 0 ? "open" : ""}>
      <summary>${escape(q)}<span class="arno">${icon("chevron")}</span></summary>
      <div class="body">${a}</div>
    </details>`).join("");
  return `
    <p class="help-intro">Quick answers to get the most out of Drift.</p>
    ${items}
    <div class="help-links">
      <a class="lk" data-url="${GH_REPO}">${icon("link")} Full guide on GitHub</a>
      <a class="lk" data-url="${GH_ISSUES}">Report an issue</a>
    </div>`;
}

function wireHelpBody(panel) {
  panel.querySelectorAll(".help-links .lk").forEach(a => a.onclick = () => {
    invoke("open_url", { url: a.dataset.url }).catch(e => showToast("error", friendlyError(e)));
  });
}

async function renderAboutBody() {
  let version = "";
  try { version = await invoke("app_version"); } catch (e) { version = ""; }
  return `
    <div class="about">
      <div class="glyph">${icon("wave")}</div>
      <h3>Drift</h3>
      <div class="ver">${version ? "Version " + escape(version) : ""}</div>
      <p class="tag">A clean, fast, native Windows torrent client with a warm, Claude-inspired interface.</p>
      <div class="about-links">
        <a class="lk" data-url="${GH_REPO}">${icon("link")} GitHub repository</a>
        <a class="lk" data-url="${GH_RELEASES}">${icon("link")} Releases · check for updates</a>
        <a class="lk" data-url="${GH_LICENSE}">${icon("link")} License (MIT)</a>
      </div>
      <div class="credits">Built with Tauri 2 · Rust · librqbit<br>Not affiliated with Anthropic — just a fan of the aesthetic.</div>
    </div>`;
}

function wireAboutBody(panel) {
  panel.querySelectorAll(".about-links .lk").forEach(a => a.onclick = () => {
    invoke("open_url", { url: a.dataset.url }).catch(e => showToast("error", friendlyError(e)));
  });
}
```

> Notes:
> - `HELP_ITEMS` answers contain trusted static `<b>`/entity markup, so the
>   answer body is inserted as-is; the question text is run through `escape()`.
> - `icon("chevron")`, `icon("link")`, `icon("wave")` all already exist in `icons.js`.

- [ ] **Step 2: Verify parse**

Run: `node --check src/main.js`
Expected: exit 0.

- [ ] **Step 3: Manual smoke**

Run: `export PATH="/c/Users/ramap/.cargo/bin:$PATH" && cd "D:/Personal Project/Drift/src-tauri" && cargo run`. Open Settings:
- **Help** tab: intro + 8 entries (first open, chevron rotates on toggle). Click **Full guide on GitHub** and **Report an issue** → each opens the browser to the right page.
- **About** tab: wave icon, "Drift", **Version 0.4.1** (matches Cargo), tagline, three links each opening the correct GitHub page, credits line.
- Switching tabs keeps the drawer open; Close works on each; reopening lands on Settings.
- Check light + dark.
Close the app.

- [ ] **Step 4: Commit**

```bash
git add src/main.js
git commit -m "Implement Help guide and About views"
```

---

### Task 5: Version bump to 0.4.1 + final verification

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`

- [ ] **Step 1: Bump versions**

In `src-tauri/tauri.conf.json` set `"version": "0.4.1"`. In `src-tauri/Cargo.toml` set `version = "0.4.1"`.

- [ ] **Step 2: Full verification**

Run: `export PATH="/c/Users/ramap/.cargo/bin:$PATH" && cd "D:/Personal Project/Drift/src-tauri" && cargo test` → all pass (the `app_version` test now asserts `"0.4.1"`).
Run: `cd "D:/Personal Project/Drift" && node --check src/main.js` → exit 0.

- [ ] **Step 3: Commit (do NOT build installers or publish unless asked)**

```bash
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "Bump version to 0.4.1"
```

> Per standing instruction: do not run `cargo tauri build` or publish until asked.

---

## Self-Review (completed by plan author)

**Spec coverage:**
- Tab switcher (Settings·Help·About), reuses segmented look, default Settings, Save only on Settings → Task 3 (shell, footer per-tab) ✓
- Settings content moved verbatim, ids + save wiring preserved → Task 3 (`renderSettingsBody`/`wireSettingsBody`) ✓
- Help view: intro + 8 collapsible entries (first open, chevron rotates) + 2 GitHub links → Task 2 (CSS), Task 4 (`renderHelpBody`/`wireHelpBody`) ✓
- About view: wave icon, version (via `app_version`), tagline, 3 links, credits → Task 1 (command), Task 2 (CSS), Task 4 (`renderAboutBody`) ✓
- Commands `app_version` + `open_url` via tauri-plugin-opener → Task 1 ✓ (verified the API is `tauri_plugin_opener::open_url(url, None::<&str>)`, mirroring existing `open_path`; `opener:default` capability already granted)
- Testing: `app_version` unit test (Task 1); manual smoke (Tasks 3,4) ✓
- Version 0.4.1 → Task 5 ✓

**Placeholder scan:** No TBD/TODO. Task 3 intentionally adds clearly-labeled temporary stubs that Task 4 replaces (each step says so). All code blocks are complete.

**Type/name consistency:** `settingsTab`, `renderSettingsPanel`, `renderSettingsBody`, `wireSettingsBody`, `renderHelpBody`, `wireHelpBody`, `renderAboutBody`, `wireAboutBody`, `HELP_ITEMS`, `GH_REPO/RELEASES/LICENSE/ISSUES`, command names `app_version`/`open_url`, and CSS classes `.settings-tabs`/`.tab`/`.help-item`/`.arno`/`.help-links .lk`/`.about`/`.about-links .lk` are used consistently across tasks. The old single `toggleSettings` body and its inline `s-cancel`/`s-save` wiring are fully replaced; `s-cancel` is now created per-tab in the shell.
