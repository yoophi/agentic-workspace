---
okf_version: "0.1"
---

# Files

- [Agent execution and orchestration flow](agent-run-flow.md) - How Agentic Workbench launches ACP runs, streams run events, mediates permissions and MCP access, and coordinates read-only child tasks through the Main Coordinator.
- [Agentic Workbench (AW)](agentic-workbench.md) - Agentic Workbench is the primary Tauri desktop application for Git worktree sessions, ACP agent runs, and Main Coordinator-led read-only agent orchestration.
- [Architecture](architecture.md) - The monorepo architecture, including its pnpm and Cargo workspaces, Feature-Sliced Design frontend, hexagonal Tauri backends, shared packages, persistence, and events.
- [Agentic Workspace quickstart](quickstart.md) - A practical starting guide to the Agentic Workspace monorepo, including the Agentic Workbench desktop application, shared packages, and SpecKit workflow.
- [Shared packages and crates](shared-packages.md) - Shared Rust crates and TypeScript packages that provide Git, ACP runtime, Markdown annotation, UI, and refresh capabilities to applications in the monorepo.
- [Spec-driven development with SpecKit](spec-workflow.md) - The repository's SpecKit workflow, artifact structure, and recent specifications for Agentic Workbench workspace, orchestration, rendering, and appearance work.
