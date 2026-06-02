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
#
# NOTE: createUpdaterArtifacts is enabled, so `cargo tauri build` SIGNS the
# updater bundle and needs a signing key in the environment. For local/test
# builds, generate a throwaway key WITH a password once:
#   cargo tauri signer generate -p "your-pass" -w "$HOME/.tauri/drift-dev.key"
# then build with it exported (PowerShell):
#   $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$HOME/.tauri/drift-dev.key" -Raw
#   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-pass"
#   cargo tauri build
# IMPORTANT: use a NON-EMPTY password. In PowerShell, `$env:VAR = ""` actually
# *unsets* the variable, so an empty password makes the signer fall back to an
# interactive prompt and the build hangs forever in a non-interactive shell.
# (Test builds signed with a throwaway key won't be accepted by the in-app
# updater — only official releases signed with the maintainer's key are. Day-to-
# day development uses `cargo tauri dev`, which needs no signing.)
cargo tauri build
```

## Releases & the auto-updater

Drift ships an in-app updater (`tauri-plugin-updater`) that polls
`latest.json` on the GitHub Releases "latest" download URL and offers to install
newer **signed** builds. The signing keypair is *not* in the repo:

- The **public** key lives in `tauri.conf.json > plugins.updater.pubkey`.
- The **private** key is the maintainer's (`~/.tauri/drift.key`) and is provided
  to CI as the `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  repo secrets (see `.github/workflows/release.yml`).

Cutting a release: bump the version in `tauri.conf.json` **and** `package.json`
(keep them in sync), update `CHANGELOG.md`, then push a `vX.Y.Z` tag. The Release
workflow builds, signs, generates `latest.json`, and creates a draft release.

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
