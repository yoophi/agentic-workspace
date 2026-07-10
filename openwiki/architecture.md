# 아키텍처

이 모노레포는 두 가지 빌드 시스템으로 구성됩니다: 프론트엔드는 **pnpm workspace + Turbo**, 백엔드는 **Cargo workspace**. 두 워크스페이스는 독립적으로 동작하지만 Tauri 앱에서 만납니다.

## 워크스페이스 구성

### pnpm workspace

`pnpm-workspace.yaml`이 `apps/*`와 `packages/*`를 워크스페이스 패키지로 등록합니다. Turbo가 작업 의존성 그래프를 관리합니다.

```text
apps/*           → Tauri 데스크톱 앱 3개 (프론트엔드 + Rust 백엔드)
packages/*       → 크로스앱 공유 TypeScript 라이브러리 6개
```

Turbo 작업 (`turbo.json`): `build`, `check-types`, `test`, `dev`, `preview`, `storybook`, `build-storybook`, `tauri`, `tauri:dev`. `build`/`check-types`/`test`는 `^` 의존성을 먼저 실행합니다.

### Cargo workspace

`Cargo.toml`이 공유 crate와 세 Tauri 앱의 Rust 백엔드를 하나의 workspace로 묶습니다:

```toml
members = [
    "crates/*",                                  # git-core
    "apps/agentic-workbench/src-tauri",
    "apps/git-explorer/src-tauri",
    "apps/markdown-annotator/src-tauri",
]
```

이를 통해 `cargo check`/`cargo test`를 workspace 전체에 한 번에 실행할 수 있습니다.

## 프론트엔드 아키텍처 — Feature-Sliced Design (FSD)

모든 앱의 프론트엔드는 Feature-Sliced Design 계층 구조를 따릅니다 (`AGENTS.md`에 명시된 규칙).

```text
src/
  app/          앱 컴포지션, 라우팅, 글로벌 상태
  pages/        화면 단위 UI
  features/     사용자 액션과 비즈니스 인터랙션
  entities/     도메인 모델, API 어댑터, 도메인 헬퍼
  shared/       크로스도메인 유틸리티, UI 프리미티브
  components/ui/  shadcn/ui 생성 컴포넌트
```

**계층 규칙**:
- `app`은 전역 라우팅과 상태만 관리
- `pages`는 화면 조립
- `features`는 사용자 행동 단위의 로직과 UI
- `entities`는 도메인 타입과 API 호출 래퍼
- `shared`는 특정 도메인에 종속되지 않는 재사용 코드
- shadcn/ui 컴포넌트는 항상 `components/ui`에서 import
- 재사용 컴포넌트는 Storybook에 등록 (atomic design: atoms → molecules → organisms → pages)

**크로스앱 공유**: 특정 Tauri 앱 셸에 의존하지 않는 TypeScript 모듈은 `packages/*`로 승격합니다. 승격 기준은 최소 2개 이상의 소비자.

## 백엔드 아키텍처 — 헥사고날 (포트와 어댑터)

모든 Tauri 백엔드는 헥사고날 아키텍처를 따릅니다 (`AGENTS.md`에 명시된 규칙).

```text
src-tauri/src/
  domain/          순수 도메인 모델과 포트 (의존성 없음)
  application/     유스케이스와 비즈니스 규칙 (포트 + 도메인에만 의존)
  inbound/         Tauri 명령 핸들러 (얇은 전달 계층)
  infrastructure/  아웃바운드 어댑터 (JSON 파일 저장, CLI 래퍼, ACP 등)
  ports/           트레이트 정의 (인터페이스)
  lib.rs           앱 조립, Tauri Builder, 메뉴, 창 라이프사이클
```

**핵심 원칙**:
- `domain`은 Tauri, 파일시스템 API, JSON 저장소에 의존하지 않음
- `inbound`의 Tauri 명령은 비즈니스 로직을 직접 구현하지 않고 application 서비스에 위임
- 영속성 로직은 infrastructure 어댑터에만 존재
- 포트(트레이트)를 통해 의존성 역전

## Tauri 앱 구조

각 Tauri 앱은 프론트엔드(`src/`)와 백엔드(`src-tauri/`)로 구성됩니다:

| 앱 | 패키지명 | 프론트엔드 | 백엔드 |
|----|---------|-----------|--------|
| Agentic Workbench | `@yoophi/agentic-workbench` | React + FSD | Rust 헥사고날, ACP 엔진 |
| Markdown Annotator | `@yoophi/markdown-annotator` | React + FSD | Rust 헥사고날 |
| Git Explorer | `@yoophi/git-explorer` | React + FSD | Rust 헥각고날 + `git-core` |

## 공유 전략

Git 관련 기능과 Markdown 주석 기능은 모노레포 전체에서 공유됩니다:

```text
Git 스택:
  crates/git-core (Rust 도메인 + CLI)
    ↕
  packages/git-graph (TS 타입 미러 + 그래프 레이아웃)
  packages/git-ui (React 컴포넌트)
    ↕
  apps/git-explorer, apps/agentic-workbench

Markdown 주석 스택:
  packages/markdown-annotation-core (프레임워크 무관 TS)
  packages/markdown-annotation-react (React 컴포넌트)
    ↕
  apps/markdown-annotator, apps/agentic-workbench
```

공유 승격은 "최소 2개 소비자" 원칙을 따릅니다. 단일 소비자 코드는 앱 내부에 유지하다가 두 번째 소비자가 생기면 `packages/` 또는 `crates/`로 추출합니다.

## 데이터 저장

AW와 GE는 JSON 파일 기반 영속성을 사용합니다 (Rust `infrastructure/json_*.rs`). 데이터베이스는 없습니다. 각 리포지토리 어댑터가 JSON 파일을 읽고 씁니다.

주요 저장 대상:
- 프로젝트 목록 (`json_project_repository`)
- worktree별 목표 (`json_goal_repository`)
- 저장 프롬프트 (`json_saved_prompt_repository`)
- 에이전트 실행 설정 (`json_agent_run_settings_repository`)
- ACP 세션 기록 (`json_acp_session_store`)
- Git 저장소 목록 — GE (`json_repository_store`)

## 이벤트 통신

Tauri 이벤트 시스템으로 백엔드→프론트엔드 통신:

- `agent-run-event`: ACP 실행 이벤트 스트림 (메시지, 툴, 권한 등)
- `workspace://worktree-changed`: 파일시스템 감시 기반 worktree 변경 알림
- `workspace://mcp-window-title`: MCP 툴에서 발생한 창 제목 변경 요청
- `repository-changed` (GE): 저장소 변경 알림

→ 에이전트 실행 흐름의 이벤트 세부사항은 [에이전트 실행 흐름](agent-run-flow.md)을 참조.
