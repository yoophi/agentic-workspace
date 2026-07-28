<!--
Sync Impact Report
Version change: 1.0.0 → 1.0.1 (PATCH)
Date: 2026-07-29
Modified principles:
- III. Hexagonal Tauri Backend Architecture — clarified port placement only.
  Previously: "Pure domain models and ports belong in `domain`."
  Now: ports MAY live in `domain` or in a dedicated top-level `ports` module,
  chosen consistently per app; a `ports` module holds port definitions and their
  signature data types only, never adapters; `domain` and `ports` both MUST stay
  free of Tauri/filesystem/persistence/UI dependencies.
Added sections: none
Removed sections: none
Rationale for PATCH: no obligation is added or removed. The technology-
independence requirement for ports is unchanged; only the permitted file
location is stated explicitly. This resolves a documented mismatch between the
constitution wording and the established repository convention (top-level
`ports` module in `apps/agentic-workbench/src-tauri/src`, in use since commit
b5f16ac), recorded as C1 in specs/033-agent-orchestration/plan.md.
Templates and docs requiring updates:
- ✅ .specify/templates/plan-template.md (already allowed a separate `ports`
  module; no change needed)
- ✅ .specify/templates/tasks-template.md (already listed `ports` as a sibling
  layer; no change needed)
- ✅ .specify/templates/spec-template.md (no port-placement guidance; no change
  needed)
- ✅ .specify/templates/checklist-template.md (no port-placement guidance; no
  change needed)
- ✅ .specify/templates/commands/*.md (directory absent; no files to update)
- ✅ AGENTS.md (updated to match the clarified rule)
- ✅ README.md (backend layer listing updated)
Follow-up TODOs:
- specs/033-agent-orchestration/plan.md: C1 deviation is resolved by this
  amendment; the Complexity Tracking entry and both Constitution Check sections
  must be updated to reflect compliance.

Previous report (1.0.0)
Version change: template → 1.0.0
Modified principles:
- PRINCIPLE_1_NAME placeholder → I. Monorepo Boundary First
- PRINCIPLE_2_NAME placeholder → II. Feature-Sliced Frontend Architecture
- PRINCIPLE_3_NAME placeholder → III. Hexagonal Tauri Backend Architecture
- PRINCIPLE_4_NAME placeholder → IV. Shared Core Before Shared UI
- PRINCIPLE_5_NAME placeholder → V. Atomic Cross-App Verification
Added sections:
- Engineering Standards
- Project Structure
Removed sections:
- Placeholder SECTION_2_NAME
- Placeholder SECTION_3_NAME
Templates requiring updates:
- ✅ .specify/templates/plan-template.md
- ✅ .specify/templates/spec-template.md
- ✅ .specify/templates/tasks-template.md
- ✅ .specify/templates/checklist-template.md
- ✅ .specify/templates/commands/*.md (directory absent; no files to update)
- ✅ README.md
Follow-up TODOs:
- None
-->
# Agentic Workspace Constitution

## Core Principles

### I. Monorepo Boundary First
App-specific code MUST live under `apps/*`. Reusable TypeScript code MUST live
under `packages/*`. Reusable Rust code MUST live under `crates/*`. Code MUST NOT
import directly from another app; cross-app reuse MUST go through a workspace
package or crate.

Shared modules MUST be introduced only when at least two apps consume them or
when a common fixture/test can verify that the module is genuinely reusable.
This keeps shared code deliberate and prevents unstable app-specific logic from
becoming a cross-app dependency.

### II. Feature-Sliced Frontend Architecture
App frontend code MUST be written under `apps/*/src` using Feature-Sliced
Design. The `app` layer owns composition, providers, and routing state.
The `pages` layer owns screen-level UI. The `features` layer owns user actions
and business interactions. The `entities` layer owns domain models, API
adapters, and domain helpers. The `shared` layer owns reusable cross-domain
utilities and UI primitives.

Generated shadcn/ui components MUST remain under `components/ui` and be
imported from there. Reusable components MUST be designed for reuse instead of
being coupled to a single screen unless the component is truly screen-specific.

### III. Hexagonal Tauri Backend Architecture
Tauri backend code MUST live under `apps/*/src-tauri/src` and MUST preserve
hexagonal boundaries. Pure domain models belong in `domain`. Use cases and
business rules belong in `application`. Tauri commands and other inbound
adapters belong in `inbound`. Filesystem, Git, ACP, JSON persistence, CLI, and
OS adapters belong in `infrastructure`.

Ports MUST be pure abstractions. An app MAY place them in `domain` alongside the
models they describe, or in a dedicated top-level `ports` module when the app has
enough ports that a separate module aids navigation. Whichever location an app
chooses, it MUST use that location consistently: a single app MUST NOT split
ports across both. A `ports` module MUST contain only port definitions and the
data types that appear in their signatures; adapter implementations MUST NOT live
there.

The `domain` layer and any `ports` module MUST NOT depend on Tauri, filesystem
APIs, JSON persistence, or UI concerns. Tauri commands MUST NOT contain
persistence or business logic; they MUST validate command input at the boundary
and delegate behavior to application services through ports/adapters.

### IV. Shared Core Before Shared UI
Git, Markdown annotation, ACP/workbench, and similar capabilities MUST share
pure core first: models, parsers, formatters, graph layout, query normalization,
state reducers, and fixture-driven tests. UI MUST be shared only after design
system and interaction requirements have demonstrably converged.

Shared UI packages MUST NOT depend on an app shell, Tauri commands, route state,
or app-local persistence. When requirements diverge, apps MUST keep UI local and
share only headless hooks, models, and helpers.

### V. Atomic Cross-App Verification
Changes to `packages/*` or `crates/*` MUST include verification for affected
consumer apps. A shared package/crate change is incomplete until the relevant
package tests and consuming app type checks, tests, or Rust checks have passed
or have been explicitly documented as not applicable.

Examples: `crates/git-core` changes require `git-core` tests and relevant Tauri
app Rust checks. `packages/markdown-annotation-*`, `packages/git-*`, and
`packages/ui` changes require the package test/typecheck plus verification of
each consuming app.

## Engineering Standards

Documentation MUST be stored in `docs/*.md`, use English file names, and write
content in Korean. Architecture and workflow documents SHOULD include Mermaid
diagrams when a flow or module relationship is being described. Design documents
MUST identify scope, non-scope, implementation phases, completion criteria, and
verification steps.

Reusable UI components MUST be registered in Storybook. Stories MUST be managed
using atomic design categories: atoms, molecules, organisms, and pages. Stateful
UI SHOULD include representative loading, empty, error, and long-content states
when those states are relevant to the component contract.

Pure logic MUST have unit tests before or alongside implementation. Parsers,
formatters, graph layout, reducers, and cross-app shared behavior MUST use
fixture-based tests where realistic input matters. Feature completion requires,
at minimum, the relevant package/app `check-types` and `test` commands. Tauri
backend changes MUST include Rust `test` or `check` for the affected crate/app.

Persistence models MUST sit behind repositories or ports. File access MUST
enforce root/path validation, size limits, and UTF-8 handling at the
application/infrastructure boundary. Agent, session, permission, and exchange
features MUST validate run/session owner scope.

Product UI in `agentic-workbench` MUST prioritize the actual work surface over
marketing or landing-page composition. Operational screens SHOULD be dense,
scan-friendly, and optimized for repeated workflows across agent timeline,
prompt, worktree, Git, and Markdown tasks.

## Project Structure

The repository is a pnpm/Turbo monorepo and a Rust Cargo workspace.

```text
apps/
  agentic-workbench/     Main Tauri app for ACP worktree sessions
  git-explorer/          Git repository exploration app
  markdown-annotator/    Markdown annotation and prompt export app
packages/
  git-graph/             Shared Git graph types and layout helpers
  git-ui/                Shared Git React UI components
  markdown-annotation-core/
                         Shared Markdown annotation parser, model, formatter
  markdown-annotation-react/
                         Shared Markdown annotation React UI and hooks
  ui/                    Shared React UI primitives
crates/
  git-core/              Shared Rust Git history/graph/detail/diff core
docs/                    Korean project documentation with English filenames
examples/                Non-workspace examples and sample documents
```

`apps/*` and `packages/*` are pnpm workspace members. `crates/*`,
`apps/agentic-workbench/src-tauri`, and `apps/git-explorer/src-tauri` are Cargo
workspace members. `apps/markdown-annotator/src-tauri` is currently app-local
and outside the root Cargo workspace.

## Governance

This constitution supersedes conflicting project habits and generated template
defaults. `AGENTS.md` remains the operational runtime guidance, but feature
plans, specs, tasks, and reviews MUST treat this constitution as the governing
architecture and verification contract.

Amendments MUST update this file and any affected `.specify/templates/*`,
README, or project documentation in the same change. Every amendment MUST
include a Sync Impact Report at the top of this file that records the version
change, modified principles, updated templates, and follow-up items.

Versioning follows semantic versioning. MAJOR changes remove or redefine a
principle in a backward-incompatible way. MINOR changes add a principle, add a
governance section, or materially expand required practice. PATCH changes
clarify wording without changing obligations.

Compliance review is required during planning and before implementation is
considered complete. Any violation MUST be recorded in the implementation plan's
Complexity Tracking table with a concrete justification and the simpler
alternative that was rejected. Architecture boundary violations are not complete
work until corrected or explicitly amended into this constitution.

**Version**: 1.0.1 | **Ratified**: 2026-07-01 | **Last Amended**: 2026-07-29
