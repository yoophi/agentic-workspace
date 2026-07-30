# Repository Instructions

## OpenWiki

- Read `openwiki/quickstart.md` before working in this repository.
- Follow its links to the relevant architecture, workflow, domain, operations, integrations, testing, and source-map documentation.
- Do not hand-edit generated OpenWiki pages unless explicitly requested.

## Release Versioning

- Use Calendar Versioning in the form `YYYY.WW.INCR`.
- `YYYY` is the ISO week-numbering year, `WW` is the ISO week number, and `INCR` is the release sequence within that ISO week.
- Start `INCR` at `0` for the first release of an ISO week and increment it for each additional release in the same week.
- Reset `INCR` to `0` when the ISO week changes.
- Write the ISO week without a leading zero so the version remains compatible with Cargo, npm, and Tauri SemVer parsing. For example, use `2026.8.0`, not `2026.08.0`.
- Name release branches `release/YYYY.WW.INCR` and release tags `vYYYY.WW.INCR`.
- Keep the root package, released app package, Tauri configuration, and Rust crate versions synchronized.
