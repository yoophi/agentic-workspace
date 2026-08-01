<!-- OPENWIKI:START -->

## OpenWiki

This repository uses OpenWiki for recurring code documentation. Start with `openwiki/quickstart.md`, then follow its links to architecture, workflows, domain concepts, operations, integrations, testing guidance, and source maps.

The scheduled OpenWiki GitHub Actions workflow refreshes the repository wiki. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->

## Versioning

- Use calendar versioning (CALVER) for AW, MA, GE, and Hushline release artifacts.
- Use the `YYYY.M.D` format for stable releases and `YYYY.M.D-rc.N` for release candidates, for example `2026.8.1-rc.1`.
- Apply CALVER only when producing a release build by overriding the Tauri bundle version at build time.
- Do not change versions in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, or any `Cargo.lock` merely to produce a CALVER build.
- Keep all desktop application artifacts on the same CALVER during a coordinated workspace release unless the user explicitly requests independent versions.
