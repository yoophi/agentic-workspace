# Agentic Workspace — 퀵스타트

Agentic Workspace는 에이전트 기반 소프트웨어 개발을 지원하는 로컬 데스크톱 도구 모노레포입니다. 핵심 앱인 **Agentic Workbench(AW)** 는 로컬 코딩 프로젝트, Git worktree, ACP(Agent Client Protocol) 에이전트 세션을 관리합니다.

## 저장소 개요

```text
apps/
  agentic-workbench/      메인 Tauri 데스크톱 앱 (프로젝트 · worktree · ACP 에이전트 실행)
  markdown-annotator/     Markdown 주석 도구 → 구조화된 프롬프트 내보내기
  git-explorer/           Git 저장소 탐색 UI
packages/
  ui/                     공유 React UI 프리미티브 (shadcn/ui 스타일)
  git-graph/              Git 커밋 그래프 타입 + 레이아웃 알고리즘 (TypeScript)
  git-ui/                 공유 Git UI 컴포넌트 (히스토리 그래프, diff 뷰어 등)
  markdown-annotation-core/   Markdown 파싱 · 주석 · TOC · 에이전트 프롬프트 포맷 (TypeScript)
  markdown-annotation-react/  Markdown 뷰어 · 주석 오버레이 · Mermaid 렌더 (React)
  workspace-auto-refresh/     파일시스템 감시 기반 자동 새로고침 훅
crates/
  git-core/               공유 Rust crate — Git 도메인 타입 · 포트 · CLI 구현
docs/                     아키텍처 및 기능 설계 문서 (한국어)
specs/                    SpecKit 기반 스펙 주도 개발 산출물
```

### 앱 별칭

| 약어 | 앱 |
|------|-----|
| AW | `apps/agentic-workbench` |
| MA | `apps/markdown-annotator` |
| GE | `apps/git-explorer` |

## 기술 스택

- **프론트엔드**: React 19, Vite, TypeScript, React Router (HashRouter), TanStack Query
- **스타일**: Tailwind CSS 4, shadcn/ui 스타일 프리미티브
- **백엔드**: Tauri 2 (Rust)
- **에이전트 통신**: `agent-client-protocol` (ACP)
- **빌드**: pnpm workspace + Turbo (프론트엔드), Cargo workspace (Rust)
- **패키지 매니저**: pnpm 9.10.0 (Corepack)

## 시작하기

### 사전 요구사항

- Node.js (Corepack 포함)
- pnpm 9.10.0
- Rust toolchain
- Tauri 2 데스크톱 필수 구성요소 (운영체제별)
- Git

### 설치

```sh
corepack enable
pnpm install
```

### 개발 서버 실행

기본 데스크톱 앱 (Agentic Workbench):

```sh
pnpm run tauri:dev          # 또는 pnpm run tauri:dev:workbench
```

각 앱별 Tauri 데스크톱 앱:

```sh
pnpm run tauri:dev:workbench
pnpm run tauri:dev:annotator
pnpm run tauri:dev:git
```

Markdown Annotator 독립 앱 번들 생성:

```sh
pnpm run tauri:build:annotator
```

산출물은 `target/release/bundle/` 아래의 운영체제별 디렉터리에 생성됩니다.

프론트엔드만 실행:

```sh
pnpm run dev:workbench
pnpm run dev:annotator
pnpm run dev:git
```

Storybook:

```sh
pnpm run storybook:annotator
pnpm run storybook:git
```

### 검증

```sh
pnpm run checktypes    # TypeScript 타입 검사
pnpm run test          # 전체 테스트
pnpm run build         # 프로덕션 빌드
```

Rust 검증은 해당 `apps/*/src-tauri` 디렉토리에서 `cargo check` 실행.

## Agentic Workbench가 하는 일

- 로컬 프로젝트를 이름, 작업 디렉토리, 설명과 함께 저장
- 각 프로젝트 디렉토리에서 Git remote, 브랜치, worktree를 읽음
- 데스크톱 UI에서 Git worktree 생성 및 삭제
- worktree 세션 페이지를 열어 ACP 에이전트 실행
- ACP 실행 출력, 툴 업데이트, 권한 프롬프트를 UI로 스트리밍
- worktree별 목표(ThreadGoal), 저장 프롬프트, 프로바이더 세션, 실행 설정 추적
- 로컬 MCP 서버를 통해 에이전트에게 worktree 컨텍스트 제공

## 주요 개념

- **ACP (Agent Client Protocol)**: 에이전트 클라이언트와 통신하는 JSON-RPC 프로토콜. AW는 에이전트를 서브프로세스로 실행하고 stdin/stdout으로 ACP 메시지를 주고받습니다.
- **Worktree 세션**: Git worktree 하나에 대응하는 에이전트 실행 단위. 세션 창에서 에이전트 실행, 파일 탐색, 변경사항 리뷰를 동시에 할 수 있습니다.
- **Permission Mode**: 에이전트의 권한 수준 (`Default`, `Auto`, `ReadOnly`, `Plan`, `AcceptEdits`, `DangerouslySkipAllPermissions`).
- **Ralph Loop**: 목표 달성 시까지 에이전트에게 자동으로 후속 프롬프트를 보내는 반복 실행 모드.
- **ThreadGoal**: worktree별 스레드 목표 — 상태 추적 및 토큰 예산 관리.

## 문서 탐색

| 문서 | 내용 |
|------|------|
| [아키텍처](architecture.md) | 모노레포 구조, Feature-Sliced Design, 헥사고날 아키텍처 원칙 |
| [Agentic Workbench](agentic-workbench.md) | 메인 앱의 프론트엔드/백엔드 구조 상세 |
| [에이전트 실행 흐름](agent-run-flow.md) | ACP 실행, 권한, MCP, 세션 관리 심층 분석 |
| [공유 패키지](shared-packages.md) | `crates/git-core`, `packages/*` 공유 모듈 |
| [스펙 주도 개발](spec-workflow.md) | SpecKit 워크플로와 `specs/` 구조 |
