# Quickstart: 프로젝트 대시보드 시작화면 검증

## Prerequisites

- Install workspace dependencies with `pnpm install` if needed.
- Work from repository root.
- Use AW app commands unless a task explicitly changes shared packages.

## Validation Scenarios

### 1. Dashboard route is the start screen

1. Run `pnpm --filter @yoophi/agentic-workbench check-types`.
2. Start the app or dev server through the existing AW workflow.
3. Open route `/`.
4. Expected: the first screen is a project dashboard, not a marketing page and not only a raw project table.

### 2. Recent projects are actionable

1. Use a project fixture or local project data with at least two projects.
2. Open the dashboard.
3. Expected: recent/relevant projects appear in the primary dashboard area.
4. Select a project action.
5. Expected: the existing project detail or worktree/session route opens correctly.

### 3. Empty state starts work clearly

1. Use Storybook or test fixture state with an empty project list.
2. Render the dashboard empty state.
3. Expected: create project and open existing project actions are visible and usable.

### 4. Loading and error states are distinct

1. Render dashboard loading state in Storybook.
2. Expected: loading is not described as "no projects".
3. Render dashboard project-list error state.
4. Expected: a retry action and short error explanation are visible.

### 5. Partial summaries degrade safely

1. Render a dashboard state where projects load but worktree/session/change summaries fail for one project.
2. Expected: project entry remains visible, primary project action remains available, unavailable summary is distinct from clean/no-session states.

### 6. Long content does not break layout

1. Render Storybook state with long project names, paths, and session labels.
2. Expected: text does not overlap buttons or adjacent status content.
3. Expected: full values remain inspectable through the established overflow pattern.

## Suggested Commands

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/agentic-workbench storybook
```

If backend commands or Rust application services are changed:

```bash
cargo test -p agentic-workbench
```

If the Cargo package name differs in this worktree, use the existing app-local Tauri/Cargo check command documented by the implementation tasks.

## Expected Artifacts After Implementation

- Dashboard page under `apps/agentic-workbench/src/pages/project-dashboard`.
- Dashboard stories in `apps/agentic-workbench/src/stories/pages.stories.tsx`.
- Unit tests for dashboard sorting/summary mapping helpers.
- Optional backend tests only when new backend query behavior is introduced.
