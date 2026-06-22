# ACP Minimal App

Minimal Tauri desktop app for managing local coding projects and running ACP
agents inside selected Git worktrees.

## What it does

- Stores local projects with a name, working directory, and optional description.
- Reads Git remotes, branches, and worktrees from each project directory.
- Creates and deletes Git worktrees from the desktop UI.
- Opens a worktree session page for running an ACP agent against that workspace.
- Streams ACP run output, tool updates, and permission prompts through the app UI.

## Tech stack

- pnpm workspace with Turbo
- Tauri 2 backend in Rust
- React 19, Vite, TypeScript, React Router, and TanStack Query
- Tailwind CSS 4 and shadcn/ui-style primitives
- `agent-client-protocol` for ACP integration

## Project layout

```text
apps/desktop/
  src/          React app using Feature-Sliced Design
  src-tauri/    Tauri backend using hexagonal architecture
```

Frontend code is organized into `app`, `pages`, `features`, `entities`, and
`components/ui`. Backend code keeps domain models and ports separate from Tauri
commands, application use cases, and infrastructure adapters.

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

Run the Tauri desktop app:

```sh
pnpm run tauri:dev
```

Run only the Vite frontend:

```sh
pnpm run dev
```

## Validation

```sh
pnpm run check-types
pnpm run build
```

## ACP agents

The backend discovers available agents through its configurable agent catalog.
Agent runs execute in the selected worktree path, so register a project first,
open one of its worktrees, and start the run from the worktree session screen.
