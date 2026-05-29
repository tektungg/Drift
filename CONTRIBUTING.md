# Contributing to Drift

Thanks for your interest! Drift is a small, personal-scale native Windows
torrent client built with Tauri 2 + Rust + a vanilla HTML/CSS/JS frontend.

## Ground rules

- **One change per PR.** Keep diffs focused and reviewable.
- **Discuss big changes first.** Open an issue before large features or
  refactors so we agree on the approach.
- **No bundled binaries or media in PRs** beyond what the app needs.

## Project layout

```
src/                      Frontend (HTML/CSS/JS, no build step; CDN ESM imports)
  main.js                 App logic
  list-ops.js             Pure list helpers (search/sort) — unit-tested
  list-ops.test.js        Node test-runner tests
  icons.js, styles.css
src-tauri/                Rust backend (Tauri app)
  src/                    commands, engine, queue, state, settings, …
  vendor/librqbit/        Vendored, lightly-patched librqbit (Apache-2.0)
docs/superpowers/         Design specs + implementation plans
```

## Prerequisites

- [Rust](https://rustup.rs/) (stable) and the
  [Tauri CLI](https://tauri.app/start/prerequisites/):
  `cargo install tauri-cli --version "^2.0" --locked`
- Visual Studio Build Tools with the "Desktop development with C++" workload
- [Node.js](https://nodejs.org/) (only for the frontend unit tests — the app
  itself has no Node build step)

## Develop & test

```powershell
# Run the app in dev
cargo tauri dev            # from src-tauri/, or: cargo tauri dev

# Rust tests
cd src-tauri; cargo test

# Frontend pure-logic tests
node --test src/list-ops.test.js

# Quick JS syntax check
node --check src/main.js

# Build installers (output in src-tauri/target/release/bundle/)
cargo tauri build
```

## Style

- Follow the patterns already in the file you're editing.
- Frontend logic that can be pure (no DOM/Tauri) should live in `list-ops.js`
  with a test.
- Rust: keep commands thin; put real logic in the typed modules
  (`engine`, `queue`, `state`).

## A note on the vendored librqbit

`src-tauri/vendor/librqbit` is a patched copy of librqbit 8.1.1 (Apache-2.0),
wired in via `[patch.crates-io]`. If you bump the librqbit version, the storage
patch (read-only file handles so completed files stay openable while seeding)
must be re-applied. See `src-tauri/vendor/librqbit/NOTICE` for what changed.

## Commits & PRs

- Write clear commit messages explaining the *why*.
- Fill in the PR template and check the boxes you actually verified.
