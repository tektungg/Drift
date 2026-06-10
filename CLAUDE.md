# Drift — notes for Claude

Native Windows torrent client: Tauri 2 + Rust backend, vanilla HTML/CSS/JS
frontend (no build step), vendored librqbit engine.

## Conventions — follow these without being asked

Read [`CONVENTIONS.md`](CONVENTIONS.md) and apply it to every commit, changelog
edit, and release. The essentials:

- **Commits** use Conventional Commits: `type(scope): subject` — imperative,
  lowercase, no trailing period. Types: `feat fix security perf refactor docs
  test build ci chore release`. Keep the `Co-Authored-By: Claude Fable 5
  <noreply@anthropic.com>` trailer on AI-assisted commits.
- **CHANGELOG.md** ([Keep a Changelog](https://keepachangelog.com/)): for any
  user-facing `feat`/`fix`/`security`, add a line under `## [Unreleased]` in the
  same change. Skip internal-only work.
- **Versioning** ([SemVer](https://semver.org/)): keep the version identical in
  `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json`, then
  `cargo build` to refresh `Cargo.lock`.
- **Releases are tag-triggered, never push-triggered.** Pushing to `main` only
  runs CI. A release happens **only** when a `vX.Y.Z` tag is pushed, and it
  produces a *draft* — the human publishes it. Never push a version tag unless
  explicitly asked to cut a release.

## Build & test

```powershell
cd src-tauri; cargo test          # Rust tests
node --test src/list-ops.test.js  # frontend pure-logic tests
node --check src/main.js          # JS syntax check
cargo tauri dev                   # run the app (from src-tauri/)
```

## Architecture notes

- Keep Tauri `#[command]`s thin (`commands.rs`); real logic lives in typed
  modules (`engine`, `queue`, `state`, `settings`, `seeding`).
- Pure, testable logic is preferred: `queue::decide` and `seeding::should_stop`
  are pure functions with unit tests; frontend pure helpers live in
  `list-ops.js` with tests. Add tests there rather than reaching into the DOM.
- `engine.rs` is the only module that touches `librqbit` types — keep it that way.
- See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the updater signing key and the
  vendored-librqbit patch.
