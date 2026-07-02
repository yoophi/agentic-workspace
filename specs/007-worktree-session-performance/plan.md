# Implementation Plan: Worktree Session 페이지 성능 개선

**Branch**: `feat/performance` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-worktree-session-performance/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

worktree session 페이지의 초기 진입과 사용 중 반응성을 개선한다. 조사 문서 `docs/worktree-session-loading-performance-review.md`에서 확인한 병목을 우선순위대로 해소한다: (1) 동기 Tauri command의 main thread 직렬화를 async command로 전환하고 status 계열 git 명령의 `.git/index` 되먹임을 차단, (2) session route의 blocking worktree 조회를 shell 우선 렌더링으로 분리, (3) watcher 이벤트 필터·trailing debounce·선별 invalidation, (4) Git 탭 선택 view 우선 조회와 agent query 캐시 정책, (5) graph 페이지네이션·파일 목록·렌더링의 대형 저장소 확장성. 각 단계는 계측(FR-013)으로 전후를 비교한다.

## Technical Context

**Language/Version**: TypeScript 5.x (React 19, Vite), Rust 2024 edition (Tauri 2)

**Primary Dependencies**: TanStack Query v5, react-router-dom, react-resizable-panels, Tauri 2, notify(파일 watcher), walkdir, 공유 crate `git-core`, 공유 패키지 `git-ui`/`git-graph`/`workspace-auto-refresh`

**Storage**: 없음(신규 영속 데이터 없음). 기존 JSON 저장소(agent settings/goal)는 계약 유지

**Testing**: vitest(패키지/앱 단위 테스트), cargo test(git-core, src-tauri), `pnpm check-types`

**Target Platform**: macOS 데스크톱(Tauri 2), Windows/Linux는 기존 지원 수준 유지

**Project Type**: desktop-app (pnpm/Turbo 모노레포 + Cargo workspace)

**Performance Goals**: spec SC 기준 — 세션 골격 표시 1초 이내(worktree 10개+), idle 10분간 watcher 유발 graph 재조회 0회, commit 5,000개 저장소에서 뒤 페이지 로드가 첫 페이지의 2배 이내, 1,000 row 로드 후 스크롤 반응성 유지

**Constraints**: 기존 Tauri command 계약은 소비처(agentic-workbench, git-explorer)와 호환 유지. domain/application 계층 계약 불변. 신규 외부 의존성 추가 최소화

**Scale/Scope**: worktree 10~30개 프로젝트, commit 수천 개 저장소, 파일 수만 개 monorepo를 상한 시나리오로 설정

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 앱 변경은 `apps/agentic-workbench`, Git 조회 로직 개선은 `crates/git-core`, virtualization은 `packages/git-ui`, refresh 옵션은 `packages/workspace-auto-refresh`에 배치한다. 앱 간 직접 import 없음. ([Project Structure](#project-structure))
- **Feature-Sliced Frontend Architecture**: PASS — route shell 우선 렌더링은 `app/App.tsx`+`pages/project-worktree-session`, query enabled/invalidation 정책은 `features/worktree-workspace`·`features/agent-run`, query 옵션·repository 시그니처는 `entities/*/api`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS — command async화는 `inbound/tauri_commands.rs`의 실행 방식 변경이며 application service 위임 구조를 유지한다. `--no-optional-locks`·watcher 필터·디렉터리 목록은 `infrastructure`, 새 port 메서드(옵션 파라미터)는 `domain`/`ports` 계약에 추가한다.
- **Shared Core Before Shared UI**: PASS — cursor 페이지네이션·count/refs 생략은 순수 로직인 `git-core`에 먼저 구현하고 앱 어댑터가 소비한다. git-ui virtualization은 이미 공유 중인 view 컴포넌트의 내부 개선으로, 앱 shell/Tauri 의존을 추가하지 않는다.
- **Atomic Cross-App Verification**: PASS — `git-core` 변경 시 `cargo test -p git-core` + agentic-workbench·git-explorer 양쪽 `cargo check/test`, `git-ui`/`git-graph`/`workspace-auto-refresh` 변경 시 패키지 테스트 + 소비 앱 `check-types`를 quickstart 검증 절차에 포함한다.
- **Documentation and Storybook**: PASS — `docs/worktree-session-loading-performance-review.md`의 실행 계획 상태를 단계별로 갱신한다. 신규 UI 상태(worktree 메타데이터 로딩 badge, worktree 검증 실패 상태)는 Storybook organisms 스토리에 추가한다.
- **Testing and Safety**: PASS — cursor 페이지네이션·status 파싱·watcher 필터는 fixture 기반 단위 테스트를 먼저 작성한다. 디렉터리 단위 파일 목록 command는 기존 경로 탈출 방지 검증(`resolve_worktree_path`)을 재사용하고 테스트로 고정한다.

## Project Structure

### Documentation (this feature)

```text
specs/007-worktree-session-performance/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── tauri-commands.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/
│   └── App.tsx                                  # ProjectWorktreeSessionRoute shell 우선 렌더링
├── pages/project-worktree-session/ui/
│   └── project-worktree-session-page.tsx        # 메타데이터 로딩/검증 실패 상태 표시
├── features/worktree-workspace/ui/
│   └── worktree-workspace-panel.tsx             # 선택 view enabled, 선별 invalidation, lazy 파일 트리
├── features/agent-run/ui/
│   └── agent-run-panel.tsx                      # agents/settings/goal staleTime·지연 조회
└── entities/
    ├── project/api/                             # listGitWorktrees includeStatus 옵션, query-options
    ├── worktree-git/api/                        # graph/history cursor 페이지네이션 파라미터
    └── worktree-file/api/                       # 디렉터리 단위 목록 API

apps/agentic-workbench/src-tauri/src/
├── domain/                                      # worktree provider port에 status 옵션 추가
├── application/                                 # 서비스 시그니처 전달(계약 유지)
├── inbound/
│   └── tauri_commands.rs                        # Git/파일 command async화(spawn_blocking), 계측 로그
└── infrastructure/
    ├── git_cli_worktree_provider.rs             # status lazy, --no-optional-locks
    ├── fs_worktree_watcher.rs                   # trailing debounce, 제외 목록 공유, .git 이벤트 세분화
    └── fs_worktree_file_provider.rs             # 디렉터리 단위 목록, markdown 필터

crates/git-core/src/
├── git_cli.rs                                   # cursor 페이지네이션, count/refs 첫 페이지 한정,
│                                                #   status reader --no-optional-locks
├── domain.rs                                    # GitCommitPage cursor/totalCount 옵션 필드
└── ports.rs                                     # reader port 파라미터 확장

packages/git-ui/src/                             # HistoryGraphView/CommitListView row virtualization
packages/workspace-auto-refresh/src/             # 활성 view 기반 invalidation 헬퍼(필요 시)

docs/worktree-session-loading-performance-review.md  # 실행 계획 상태 갱신
```

**Structure Decision**: 기존 구조를 그대로 사용하며 새 디렉터리를 만들지 않는다. 백엔드 성능 로직은 git-core(공유) → infrastructure(앱 어댑터) → inbound(async 실행) 순으로 배치하고, 프론트는 entities(API 옵션) → features(query 정책) → app/pages(shell 렌더링) 순으로 소비한다.

## Complexity Tracking

> Constitution Check 위반 없음 — 해당 없음.
