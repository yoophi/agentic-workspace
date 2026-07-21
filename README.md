# Agentic Workspace

Agentic Workspace is a monorepo for local desktop tools that support
agent-assisted software development. Its main app, Agentic Workbench, manages
local coding projects, Git worktrees, and ACP agent sessions.

## Apps

| App | Purpose |
| --- | --- |
| `apps/agentic-workbench` | Main desktop workbench for managing projects, Git worktrees, and ACP agent runs. |
| `apps/markdown-annotator` | Markdown annotation tool that exports structured prompts for coding agents. |
| `apps/git-explorer` | Git repository exploration UI. |
| `apps/hushline` | Local YouTube-to-Whisper transcription app; organizes transcripts and chats over them via ACP agent runs. |
| `crates/acp-agent-core` | Shared Rust core for ACP agent runs (run domain, ports, use cases, ACP adapter, session registry). |
| `packages/agent-client` | Shared TS client for agent runs (run-communication contract types + invoke/listen wrappers). |
| `packages/ui` | Shared React UI primitives used across apps. |

## What Agentic Workbench Does

- Stores local projects with a name, working directory, and optional description.
- Reads Git remotes, branches, and worktrees from each project directory.
- Creates and deletes Git worktrees from the desktop UI.
- Opens a worktree session page for running an ACP agent against that workspace.
- Streams ACP run output, tool updates, and permission prompts through the app UI.
- Tracks worktree-specific goals, saved prompts, provider sessions, and run settings.

## Tech Stack

- pnpm workspace with Turbo
- Tauri 2 backend in Rust
- React 19, Vite, TypeScript, React Router, and TanStack Query
- Tailwind CSS 4 and shadcn/ui-style primitives
- `agent-client-protocol` for ACP integration

## Project layout

```text
apps/
  agentic-workbench/     Main Tauri desktop app for ACP worktree sessions
  markdown-annotator/    Markdown annotation and prompt export app
  git-explorer/          Git repository exploration app
packages/
  ui/                    Shared React UI primitives
docs/                    Architecture and feature design notes
```

## Requirements

- Node.js with Corepack
- pnpm 9.10.0
- Rust toolchain
- Tauri desktop prerequisites for your operating system
- Git

## Setup

```sh
corepack enable
pnpm install
```

## Development

Run the default desktop app, Agentic Workbench:

```sh
pnpm run tauri:dev
```

Run each app frontend only:

```sh
pnpm run dev:workbench
pnpm run dev:annotator
pnpm run dev:git
```

Run each Tauri desktop app:

```sh
pnpm run tauri:dev:workbench
pnpm run tauri:dev:annotator
pnpm run tauri:dev:git
```

Run Storybook:

```sh
pnpm run storybook:annotator
pnpm run storybook:git
```

## Validation

```sh
pnpm run check-types
pnpm run test
pnpm run build
```

For app-specific Rust validation, run `cargo check` inside the relevant
`apps/*/src-tauri` directory.

## Architecture

Frontend code follows Feature-Sliced Design:

```text
app/       App composition and routing state
pages/     Screen-level UI
features/  User actions and business interactions
entities/  Domain models, API adapters, and domain helpers
shared/    Reusable cross-domain utilities and UI primitives
```

Tauri backend code follows hexagonal architecture:

```text
domain/          Pure domain models and ports
application/     Use cases and business rules
inbound/         Tauri commands and other inbound adapters
infrastructure/  Git, ACP, persistence, filesystem, and OS adapters
```

## ACP agents

Agentic Workbench discovers available agents through its configurable agent
catalog. Agent runs execute in the selected worktree path, so register a project
first, open one of its worktrees, and start the run from the worktree session
screen.

## Documentation

Project governance:

- `.specify/memory/constitution.md`
- `AGENTS.md`

Key design notes:

- `docs/portable-architecture-plan.md`
- `docs/agent-run-session-portability-design.md`
- `docs/ralph-mode-implementation.md`
- `docs/acp-http-websocket-transport-design.md`
- `docs/git-feature-sharing-monorepo-strategy.md`
- `docs/markdown-annotation-preparation-plan.md`
