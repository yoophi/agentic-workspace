## Personal Notes

- The user uses Logseq for personal notes.
- The private Logseq graph is located at `~/docs/private-zk`.
- When the user asks to save something as a private Logseq document, save it under `~/docs/private-zk`.

## OpenWiki

This repository has documentation located in the /openwiki directory.

Start here:
- [OpenWiki quickstart](openwiki/quickstart.md)

OpenWiki includes repository overview, architecture notes, workflows, domain concepts, operations, integrations, testing guidance, and source maps.

When working in this repository, read the OpenWiki quickstart first, then follow its links to the relevant architecture, workflow, domain, operation, and testing notes.

## Documentation

- Store project documentation under `docs/*.md`.
- Use English file names for documentation files.
- Write documentation content in Korean.
- Use Mermaid charts for diagrams and flows in documentation.

## Project Aliases

- `AW` means `apps/agentic-workbench`.
- `MA` means `apps/markdown-annotator`.
- `GE` means `apps/git-explorer`.

## Frontend Architecture

- Write app frontend code under `apps/*/src` using Feature-Sliced Design.
- Keep app composition and routing state in `app`.
- Put screen-level UI in `pages`.
- Put user actions and business interactions in `features`.
- Put domain models, domain API adapters, and domain-specific helpers in `entities`.
- Put reusable cross-domain utilities and UI primitives in `shared` when they are not shadcn/ui registry components.
- Keep shadcn/ui generated components under `components/ui` and import them from there.
- When creating components, design them for reuse instead of coupling them to one screen unless the component is truly screen-specific.
- Register reusable components in Storybook.
- Manage Storybook stories according to atomic design categories: atoms, molecules, organisms, and pages.
- Put cross-app reusable TypeScript modules under `packages/*` when they do not depend on a specific Tauri app shell.

## Tauri Backend Architecture

- Write Tauri backend code under `apps/*/src-tauri/src` using hexagonal architecture.
- Keep pure domain models in `domain`.
- Keep ports in `domain` or in a dedicated top-level `ports` module, and use one location consistently per app. `agentic-workbench` uses `ports`.
- Keep only port definitions in a `ports` module; adapter implementations belong in `infrastructure`.
- Keep use cases and business rules in `application`.
- Keep inbound adapters such as Tauri commands in `inbound`.
- Keep outbound adapters such as JSON file persistence in `infrastructure`.
- Do not let `domain` or `ports` depend on Tauri, filesystem APIs, or JSON storage details.
- Do not put persistence logic directly in Tauri commands; commands should delegate to application services through ports/adapters.

<!-- OPENWIKI:START -->

## OpenWiki

This repository uses OpenWiki for recurring code documentation. Start with `openwiki/quickstart.md`, then follow its links to architecture, workflows, domain concepts, operations, integrations, testing guidance, and source maps.

The scheduled OpenWiki GitHub Actions workflow refreshes the repository wiki. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->
