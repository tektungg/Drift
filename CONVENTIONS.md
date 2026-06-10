# Drift Conventions

The single source of truth for how we write commits, changelog entries, versions,
and releases. The goal is consistency: anyone (human or AI) reading the history or
cutting a release should produce identical-looking output without guessing.

> **TL;DR**
> - Commits follow [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): subject`.
> - `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/); user-facing changes go under `## [Unreleased]` as you make them.
> - Versions follow [SemVer](https://semver.org/) and must stay in sync across three files.
> - **A normal `git push` never releases.** Releases happen **only** when you push a `vX.Y.Z` tag.

---

## 1. Commit messages

Format (Conventional Commits):

```
type(scope): subject

Optional body explaining WHY, wrapped at ~72 columns.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
```

### Subject line

- **type** — one of the set below. Required.
- **scope** — optional, lowercase, the area touched. Use one when it sharpens the
  message: `engine`, `queue`, `state`, `settings`, `commands`, `updater`, `tray`,
  `clipboard`, `seeding`, `ui`, `ci`, `release`, `deps`.
- **subject** — imperative mood ("add", not "added"/"adds"), lowercase first
  letter, **no trailing period**, aim for ≤ 72 characters.

Good: `feat(seeding): stop seeding when ratio limit is reached`
Avoid: `Fixed the seeding bug.` (no type, past tense, trailing period)

### Types

| Type | Use for | Shows in CHANGELOG? |
|---|---|---|
| `feat` | A user-facing feature or capability | **Yes** → Added / Changed |
| `fix` | A bug fix | **Yes** → Fixed |
| `security` | A security-relevant change | **Yes** → Security |
| `perf` | A performance improvement | Usually → Changed |
| `refactor` | Code change with no behavior change | No |
| `docs` | Documentation only | No |
| `test` | Tests only | No |
| `build` | Build system, bundling, installer, deps | Only if user-visible |
| `ci` | GitHub Actions / CI config | No |
| `chore` | Housekeeping (gitignore, renames, …) | No |
| `release` | The version-bump commit only (see §4) | n/a |

### Body

- Explain **why**, not what — the diff already shows what.
- Wrap at ~72 columns.
- Reference issues with `Refs #12` or `Closes #12` when relevant.

### AI-assisted commits

Keep the trailer on any commit produced with AI assistance:

```
Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
```

### Merge commits

When merging a feature branch, use a descriptive merge subject:

```
Merge <feature>: <one-line summary>
```

e.g. `Merge 0.6.0: seeding limits, completion notifications, CSP hardening`.

### Template

A commit template lives at [`.gitmessage`](.gitmessage). Enable it once per clone:

```powershell
git config commit.template .gitmessage
```

Then `git commit` (no `-m`) opens the editor pre-filled with the format and the
type list as comments.

---

## 2. CHANGELOG

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

- **Add an entry in the same commit/PR that makes a user-facing change**, under
  the `## [Unreleased]` heading. Don't wait until release time.
- Group entries under these headings, in this order, omitting empty ones:
  **Added · Changed · Deprecated · Removed · Fixed · Security**.
- Write for **users**, not developers. Lead with what they can now do or what
  stopped going wrong. Bold the feature name.
- Internal-only changes (`refactor`, `test`, `ci`, most `chore`/`docs`) do **not**
  get a CHANGELOG entry.

Example entry:

```markdown
## [Unreleased]
### Added
- **Seed ratio limit**: stop seeding automatically once a torrent reaches a
  configurable upload ratio (Settings → Seeding).
```

At release time the `## [Unreleased]` block is renamed to
`## [X.Y.Z] — YYYY-MM-DD` and a fresh empty `## [Unreleased]` is added on top.

---

## 3. Versioning (SemVer)

`MAJOR.MINOR.PATCH`:

- **MAJOR** — incompatible/disruptive change (rare for an app; e.g. a data
  format users can't migrate from automatically).
- **MINOR** — a new user-facing feature, backward compatible. (0.5 → 0.6)
- **PATCH** — bug fixes / polish only, no new features. (0.6.0 → 0.6.1)

The version number **must be identical** in all three files, kept in sync:

1. `src-tauri/tauri.conf.json` → `"version"`
2. `src-tauri/Cargo.toml` → `version`
3. `package.json` → `"version"`

After editing `Cargo.toml`, run `cargo build` in `src-tauri/` so `Cargo.lock`
records the new version too.

---

## 4. Releases

### How triggering works (read this first)

- **`ci.yml`** runs on **every push to `main` and every PR** — it builds and runs
  tests. It does **not** release anything.
- **`release.yml`** runs **only when a tag matching `v*` is pushed**. It builds the
  signed installers, generates `latest.json`, and creates a **draft** GitHub
  Release.

➡️ **Pushing commits to `main` is always safe and never publishes a release.**
A release happens only when you deliberately push a `vX.Y.Z` tag. So you can push
work-in-progress freely; tag only when you mean to ship.

### Release checklist

1. Make sure `main` is green (CI passing) and all the release's changes are merged.
2. **Bump the version** in the three files (§3) and refresh `Cargo.lock`
   (`cargo build` in `src-tauri/`).
3. **Finalize the CHANGELOG**: rename `## [Unreleased]` to
   `## [X.Y.Z] — YYYY-MM-DD`, add a new empty `## [Unreleased]` above it, and
   update the compare-links at the bottom if present.
4. Commit it with the dedicated release type:
   ```
   release: X.Y.Z — <short summary of headline changes>
   ```
5. Push `main`:
   ```powershell
   git push origin main
   ```
   (Still no release — this only runs CI.)
6. Tag and push the tag — **this is the step that releases**:
   ```powershell
   git tag -a vX.Y.Z -m "Drift X.Y.Z — <short summary>"
   git push origin vX.Y.Z
   ```
7. Watch the build: `gh run list --workflow=release.yml`.
8. When it finishes, open the **draft** release on GitHub, review the notes
   (auto-filled from the CHANGELOG section), and click **Publish**. Publishing is
   what makes the in-app auto-updater offer the new version to existing users — so
   it's intentionally a manual human step, never automated.

### Tag format

- Always annotated (`git tag -a`), always `vX.Y.Z` (leading `v`, matching the
  `## [X.Y.Z]` CHANGELOG heading and the `version` fields without the `v`).
- One tag per released version. Never move or delete a published tag.

### If something is wrong after tagging but before publishing

The release is still a draft. Delete the draft on GitHub, delete the tag locally
and remotely, fix it, and re-tag:

```powershell
git push origin :refs/tags/vX.Y.Z   # delete remote tag
git tag -d vX.Y.Z                    # delete local tag
```

---

## 5. Branches & PRs

- Branch off `main`: `feat/<slug>`, `fix/<slug>`, `docs/<slug>`.
- **One logical change per PR** (matches CONTRIBUTING.md).
- PR title follows the same `type(scope): subject` convention as commits.
- Fill in the PR template; check only the boxes you actually verified.
- Squash-or-merge is fine; keep the final commit messages conventional.

---

See also [`CONTRIBUTING.md`](CONTRIBUTING.md) for environment setup, the vendored
librqbit note, and the updater signing-key details.
