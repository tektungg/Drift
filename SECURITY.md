# Security Policy

## Supported versions

Drift is a personal project; only the **latest release** receives fixes.
Please update to the newest version on the
[Releases page](https://github.com/tektungg/Drift/releases) before reporting.

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Instead, report privately via GitHub's
[private vulnerability reporting](https://github.com/tektungg/Drift/security/advisories/new)
("Report a vulnerability" under the repository's **Security** tab). Include:

- what the issue is and where (file/feature),
- steps to reproduce or a proof of concept,
- the Drift version (Settings → About) and your Windows version.

You'll get an acknowledgement as soon as possible. Since this is a hobby
project maintained in spare time, fixes are best-effort — but security reports
are taken seriously and prioritized over features.

## Scope notes

- Drift downloads arbitrary peer-to-peer content; **the safety of files you
  download is your responsibility**. Reports about specific torrents or content
  are out of scope.
- The magnet-handler and clipboard-watch features are opt-in and operate
  locally; reports about how they handle untrusted magnet strings are in scope.
