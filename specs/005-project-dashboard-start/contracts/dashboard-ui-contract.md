# Contract: Project Dashboard UI

## Purpose

Defines the observable behavior expected from the AW project dashboard start screen. This is a UI and app-boundary contract, not an external web API.

## Entry Point

- Route `/` displays the project dashboard.
- Existing project detail and worktree/session routes remain reachable from dashboard actions.
- Creating a project continues to use the existing project form flow.

## Inputs

The dashboard consumes:

- Project list query result.
- Optional worktree summary per project.
- Optional recent/resumable session summary per project.
- Optional change summary for project/worktree status.
- User actions: create project, open existing project, select project, resume session, open worktree, retry failed load.

## Required States

### Loading

- Shows that project data is being loaded.
- Does not present loading as "no projects".
- Keeps non-data-dependent actions available when possible.

### Ready With Recent Projects

- Shows recent or relevant projects first.
- Each item exposes a primary action to continue/open that project.
- Items display available session/worktree/change summary without requiring every summary source to be complete.

### Empty

- Shows that no projects are registered.
- Provides clear actions for creating a project and opening an existing project.

### Error

- Shows a short user-facing error summary when project list loading fails.
- Provides retry and at least one alternate project-start action when possible.

### Partial Summary

- Shows project entries even when optional worktree/session/change summary fails.
- Marks unavailable summary data distinctly from "no changes" or "no sessions".

### Long Content

- Long project names, paths, and labels do not overlap action controls.
- Full values remain inspectable through the app's established overflow pattern.

## Action Outcomes

| Action | Expected Outcome |
|--------|------------------|
| Create project | Opens the project creation flow |
| Open existing project | Opens the existing project selection/open flow |
| Select project | Navigates to that project's existing project route |
| Resume session | Navigates to or opens the resumable session context |
| Open worktree | Navigates to or opens the selected worktree context |
| Retry | Re-runs the failed project/dashboard load |

## Non-Goals

- The dashboard does not replace the full project list management screen.
- The dashboard does not expose every worktree/session detail.
- The dashboard does not introduce cross-app shared UI packages.
- The dashboard does not require a new persistence model unless implementation discovers that existing activity signals are insufficient.

## Verification Requirements

- Storybook must include ready, empty, loading, error, partial summary, and long-content examples.
- Unit tests must cover recent/relevant sorting and summary status mapping.
- If backend commands are added, command tests or Rust checks must verify boundary delegation and inaccessible path/session handling.
