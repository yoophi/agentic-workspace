## Personal Notes

- The user uses Logseq for personal notes.
- The private Logseq graph is located at `~/docs/private-zk`.
- When the user asks to save something as a private Logseq document, save it under `~/docs/private-zk`.

## Documentation

- Store project documentation under `docs/*.md`.
- Use English file names for documentation files.
- Write documentation content in Korean.
- Use Mermaid charts for diagrams and flows in documentation.

## Frontend Architecture

- Write code under `apps/agentic-workbench/src` using Feature-Sliced Design.
- Keep app composition and routing state in `app`.
- Put screen-level UI in `pages`.
- Put user actions and business interactions in `features`.
- Put domain models, domain API adapters, and domain-specific helpers in `entities`.
- Put reusable cross-domain utilities and UI primitives in `shared` when they are not shadcn/ui registry components.
- Keep shadcn/ui generated components under `components/ui` and import them from there.
- When creating components, design them for reuse instead of coupling them to one screen unless the component is truly screen-specific.
- Register reusable components in Storybook.
- Manage Storybook stories according to atomic design categories: atoms, molecules, organisms, and pages.

## Tauri Backend Architecture

- Write code under `apps/agentic-workbench/src-tauri/src` using hexagonal architecture.
- Keep pure domain models and ports in `domain`.
- Keep use cases and business rules in `application`.
- Keep inbound adapters such as Tauri commands in `inbound`.
- Keep outbound adapters such as JSON file persistence in `infrastructure`.
- Do not let `domain` depend on Tauri, filesystem APIs, or JSON storage details.
- Do not put persistence logic directly in Tauri commands; commands should delegate to application services through ports/adapters.
