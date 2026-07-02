# Implementation Plan: 워킹 트리(미커밋) 변경사항 조회 기능의 공유 패키지화 및 양 앱 통합

**Branch**: `feat/git-explorer-aw-integration` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-shared-worktree-changes/spec.md`

> **참고**: 본 plan은 이미 구현 완료된 작업(커밋 `474d923` → `c54faa0` → `34b0d65` → `183f9c0`)의 회고적 문서화이며, 잔여 작업(Storybook 등록 등)의 식별 근거로 사용한다.

## Summary

agentic-workbench(AW)에만 존재하던 미커밋(working-tree) status/diff 조회 로직을 `crates/git-core`로 승격해 단일 정본으로 만들고, TS 타입 미러(`packages/git-graph`)와 표현 전용 공유 뷰(`packages/git-ui`의 `WorktreeChangesView`)를 추가한다. git-explorer(GE)는 이 공유 계층을 소비해 미커밋 변경 조회 기능을 새로 탑재하고(Commit ↔ Working tree 토글), AW는 자체 구현을 삭제하고 공유 구현으로 교체한다. 읽기 전용 범위이며 쓰기 동작(stage/unstage/discard)은 비목표다.

## Technical Context

**Language/Version**: Rust (Cargo workspace, edition 2021) + TypeScript (pnpm/Turbo monorepo), React 18

**Primary Dependencies**: git CLI(시스템 설치 전제), Tauri 2 (커맨드 브리지), @tanstack/react-query (앱 측 데이터 페칭), lucide-react (공유 뷰 아이콘, git-ui peer)

**Storage**: N/A — 조회 전용, 영속화 없음. git CLI 출력의 파싱만 수행

**Testing**: `cargo test` (git-core 단위 7개, GE 32개, AW 104개), `pnpm check-types` (git-graph/git-ui/GE/AW)

**Target Platform**: macOS/데스크톱 (Tauri 앱 2종: agentic-workbench, git-explorer)

**Project Type**: 데스크톱 앱 2종 + 공유 crate/패키지 계층 (모노레포)

**Performance Goals**: 미커밋 변경 목록·diff가 사용자 체감상 즉시(<1s) 표시. 대용량 diff도 UI 응답성 유지

**Constraints**: diff 응답 크기 상한 `MAX_WORKTREE_DIFF_BYTES = 120,000` bytes (초과 시 잘림 + `isTruncated` 플래그). git CLI 부재/비저장소 경로 시 오류 메시지 반환(패닉 없음)

**Scale/Scope**: 소비 앱 2개, 신규 공유 컴포넌트 1개, 신규 포트 1개 + CLI 어댑터 1개, Tauri 커맨드 각 앱 2개(status/diff)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: **PASS** — 조회 로직은 `crates/git-core`(`domain.rs`, `ports.rs`, `git_cli.rs`), TS 타입은 `packages/git-graph/src/types.ts`, 공유 뷰는 `packages/git-ui/src/ui/worktree-changes-view.tsx`. 앱 코드는 `apps/git-explorer`, `apps/agentic-workbench` 내부에만 있고 앱 간 직접 import 없음. 공유 승격 요건(2개 앱 소비)도 충족 — AW(기존 기능 이관) + GE(신규 소비).
- **Feature-Sliced Frontend Architecture**: **PASS** — GE: API 어댑터·쿼리 키는 `apps/git-explorer/src/entities/repository/api.ts`, 화면 조립은 `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`. AW: `apps/agentic-workbench/src/entities/project/{api,model}` + 해당 패널 UI(`worktree-changes-panel.tsx`, `worktree-workspace-panel.tsx`). shadcn 생성 컴포넌트 위치 변경 없음.
- **Hexagonal Tauri Backend Architecture**: **PASS** — 포트 `GitWorktreeStatusReader`는 git-core `ports.rs`(도메인 순수), CLI 어댑터 `GitCliWorktreeStatusReader`는 `git_cli.rs`. GE Tauri 커맨드(`get_worktree_status`/`get_worktree_file_diff`)는 `WorktreeStatusService` facade(repositoryId→path 변환)에 위임. AW 커맨드(`get_worktree_changes`/`get_worktree_file_diff`)는 application 서비스에서 입력 정규화(`normalize_required`) 후 reader에 위임. 커맨드에 비즈니스 로직 없음.
- **Shared Core Before Shared UI**: **PASS** — 순수 코어(도메인 타입 + porcelain v1 파싱 + 단위 테스트 7개)를 먼저 git-core에 공유. UI는 두 앱의 요구(그룹 목록 + diff)가 수렴했고 기존 공유 `DiffViewer`가 이미 존재하므로 `WorktreeChangesView`로 공유. 공유 뷰는 데이터·콜백 전부 props 주입(`CommitDetailView` 패턴), Tauri/앱 셸/라우팅 비의존.
- **Atomic Cross-App Verification**: **PASS** — git-core 변경 → git-core 테스트 7개 통과. GE 소비 → Rust 테스트 32개 + check-types 통과. AW 마이그레이션 → Rust 테스트 104개 + check-types 통과. git-graph/git-ui 변경 → 양 앱 check-types에 포함.
- **Documentation and Storybook**: **PASS (implement 단계에서 보완 완료)** — `WorktreeChangesView` 스토리 6종(기본·clean·로딩·오류·바이너리·잘림)을 프로젝트 관례에 따라 `apps/git-explorer/src/stories/organisms.stories.tsx`에 등록하고 `build-storybook` 성공 확인. 구조 문서 `docs/git-worktree-changes-architecture.md` 신규 작성(Mermaid 포함).
- **Testing and Safety**: **PASS** — 순수 파싱 로직(porcelain v1)은 git-core 단위 테스트로 검증. 경로 검증은 각 앱 서비스 경계에서 수행(GE facade의 repositoryId 해석, AW의 `normalize_required`). diff 크기 상한으로 과대 응답 차단. 영속화·세션 소유권 관련 변경 없음.

**Gate 결과**: 전 게이트 PASS. (Storybook 미등록 건은 implement 단계 T024로 보완 완료.)

## Project Structure

### Documentation (this feature)

```text
specs/006-shared-worktree-changes/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── tauri-commands.md  # Phase 1 output — 포트/커맨드/컴포넌트 계약
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
# 공유 Rust 코어 (정본)
crates/git-core/src/
├── domain.rs            # GitWorktreeChanges / GitChangedFile / GitChangedFileGroup / GitWorktreeFileDiff
├── ports.rs             # GitWorktreeStatusReader (GitHistoryReader와 분리)
├── git_cli.rs           # GitCliWorktreeStatusReader (porcelain v1 파싱 + diff/--cached, 120KB 상한)
└── lib.rs               # 공개 re-export

# 공유 TypeScript
packages/git-graph/src/
└── types.ts             # Rust 도메인 타입의 TS 미러 (camelCase serde와 정합)
packages/git-ui/src/
├── ui/worktree-changes-view.tsx  # 공유 표현 전용 뷰 (props 주입)
└── index.ts             # WorktreeChangesView export

# git-explorer (신규 소비)
apps/git-explorer/src-tauri/src/
├── application/worktree_status_service.rs  # facade: repositoryId→path 변환
├── adapters/inbound/tauri_commands.rs      # get_worktree_status / get_worktree_file_diff
└── lib.rs                                  # 커맨드 등록
apps/git-explorer/src/
├── entities/repository/api.ts              # invoke 래퍼 + queryKeys
└── widgets/changes-panel/ui/ChangesPanel.tsx  # Commit/Working tree 토글 + WorktreeChangesView

# agentic-workbench (마이그레이션)
apps/agentic-workbench/src-tauri/src/
├── application/git_worktree_changes_service.rs  # git-core reader 주입 + 입력 정규화
├── inbound/tauri_commands.rs                    # get_worktree_changes / get_worktree_file_diff
└── (삭제) domain/git_worktree_changes_provider.rs, infrastructure/git_cli_worktree_changes_provider.rs
apps/agentic-workbench/src/
├── entities/project/{api,model}                 # @yoophi/git-graph re-export로 전환
└── .../ui/{worktree-changes-panel,worktree-workspace-panel}.tsx  # 공유 뷰 렌더 + Git 탭 토글
```

**Structure Decision**: 헌법 원칙 I·III·IV에 따라 "코어(crate) → 타입 미러(package) → 공유 뷰(package) → 앱 소비" 4계층으로 배치했다. 도메인·포트·어댑터는 git-core 내부에서 파일 단위(domain/ports/git_cli)로 분리하고, 앱별 식별자(repositoryId 등)→경로 변환은 각 앱의 application 계층 facade가 담당한다.

## Complexity Tracking

> 정당화가 필요한 헌법 위반 없음. Storybook 미등록은 위반 예외가 아닌 잔여 태스크로 처리한다(tasks.md 참조).
