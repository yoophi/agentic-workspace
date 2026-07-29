# Implementation Plan: Markdown 렌더링 품질 개선

**Branch**: `033-markdown-rendering-quality` | **Date**: 2026-07-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from [`specs/033-markdown-rendering-quality/spec.md`](./spec.md)

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

CommonMark 및 현재 지원 중인 GFM 문법을 문서 전체의 AST 문맥에서 해석하도록
Markdown 미리보기 경로를 재구성한다. 현재의 행 단위 블록 분해와 블록별 독립
`ReactMarkdown` 렌더링은 목록의 lazy continuation, 여러 문단을 가진 항목, 중첩
목록을 서로 분리해 유효하지 않은 HTML과 의미 손실을 만든다. `markdown-annotation-core`
에는 AST/위치 기반의 안정적인 주석 블록 모델과 fixture 품질 코퍼스를 두고,
`markdown-annotation-react`는 한 번의 CommonMark+GFM 렌더링 중 의미론적 블록에
주석 식별자·도구 모음·인라인 주석 표시를 결합한다. MA와 AW는 동일한 공유 뷰어
계약을 소비하며, Mermaid·TOC·wikilink·안전 정책을 회귀 검증한다. AW의 workspace
미리보기에는 주석 보조 영역을 접고 펼치는 화면 상태를 추가해, 읽기 중에는 본문 열이
확보된 공간을 사용하면서도 기존 주석과 작성 중인 작업을 보존한다.

## Technical Context

**Language/Version**: TypeScript 5, React 19, Node.js/pnpm workspace

**Primary Dependencies**: `react-markdown` 10, `remark-gfm` 4, unified/remark AST 도구(직접 의존성으로 명시), Vitest 4, Storybook 10, Mermaid 11

**Storage**: N/A — 문서와 주석은 기존 소비자 상태/저장 모델을 유지하며, 주석 영역 표시 상태도 현재 화면 수명 안에서만 관리하고 새 영속 모델을 만들지 않음

**Testing**: Vitest fixture 단위·렌더 HTML 구조·주석 상호작용 회귀 테스트, MA/AW 소비자 타입 검사 및 테스트, Storybook 빌드

**Target Platform**: Tauri 2 데스크톱의 Chromium WebView 및 Vite/SSR 테스트 환경

**Project Type**: 공유 TypeScript 라이브러리 + Tauri 데스크톱 앱

**Performance Goals**: 2,000개 이상 독립 문서 구조에서 기존 미리보기 기준을 유지하고, 새 fixture 전체를 일반 개발 환경에서 빠르게 실행 가능하게 유지

**Constraints**: CommonMark 핵심 + 현행 GFM만 지원; 원시 HTML/활성 URL은 안전 정책을 약화시키지 않음; 유효한 HTML 계층 유지; 기존 `MarkdownBlock` 기반 주석·TOC·Mermaid 소비 흐름 호환; AW 주석 영역 전환은 문서·주석·선택·편집 상태를 초기화하지 않음

**Scale/Scope**: 최소 20개 구조 품질 fixture, 최소 10개 주석 결합 fixture, 최소 10개 복원력 fixture, AW 주석 영역 표시/숨김 회귀 사례; 공유 패키지 2개와 소비자 앱 MA·AW

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Answer each gate with PASS/FAIL/N/A and cite the concrete files or plan
sections that justify the answer.

- **Monorepo Boundary First — PASS**: 문법 해석, fixture, 타입은
  `packages/markdown-annotation-core`에, React 렌더링과 주석 DOM 결합은
  `packages/markdown-annotation-react`에 둔다. MA/AW는 각자 소비와 화면 검증만
  수행하며 앱 간 직접 import를 추가하지 않는다.
- **Feature-Sliced Frontend Architecture — PASS**: MA는 기존
  `pages/annotator` 및 `features/open-document`의 소비 경로를 유지하고, AW는 기존
  `features/worktree-workspace` 안에서만 주석 영역 표시 상태와 제어를 조정한다. 새
  앱 횡단 UI는 만들지 않는다.
- **Hexagonal Tauri Backend Architecture — N/A**: Tauri 명령·도메인·영속소 변경이
  없다.
- **Shared Core Before Shared UI — PASS**: AST 위치, 블록 식별, fixture 기대 구조와
  주석 앵커 정합성은 순수 core에서 먼저 정의한다. React 계층은 이 결과와 표준
  `react-markdown` AST만 소비한다.
- **Atomic Cross-App Verification — PASS**: 두 공유 package의 타입/테스트와 MA·AW의
  타입/테스트를 계획의 완료 기준으로 둔다.
- **Documentation and Storybook — PASS**: `docs/markdown-rendering-quality.md`에
  한국어 품질 기준과 Mermaid 검증 흐름을 추가하고, MA의 molecule Storybook에 복합
  목록·주석·오류 복원 사례를 추가한다.
- **Testing and Safety — PASS**: CommonMark/GFM fixture 및 HTML 구조 비교를 단위
  테스트로, 주석의 블록/부분 선택 편집·취소를 렌더링 테스트로 다룬다. 원시 HTML과
  위험 URL의 비실행 동작을 fixture로 명시한다.

## Project Structure

### Documentation (this feature)

```text
specs/033-markdown-rendering-quality/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)
```text
packages/markdown-annotation-core/
├── src/parse/                 # AST 기반 문서/주석 블록 추출 및 보조 함수
├── src/types/                 # 렌더 블록·fixture 계약 타입
└── src/quality/               # 버전 관리 fixture, 기대 구조 비교, 테스트

packages/markdown-annotation-react/
├── src/MarkdownViewer.tsx     # 단일 문서 렌더와 의미론적 블록/주석 결합
├── src/markdown-components.tsx # 유효한 HTML을 보장하는 AST element adapter
└── src/*.test.tsx             # HTML 구조 및 주석 회귀 테스트

apps/markdown-annotator/
├── src/pages/annotator/       # 기존 viewer 소비 및 E2E 성격 회귀 테스트
└── src/stories/molecules/     # 재사용 viewer 품질 Storybook 사례

apps/agentic-workbench/
└── src/features/worktree-workspace/ # preview, 주석 영역 전환, 소비 검증

docs/markdown-rendering-quality.md
```

**Structure Decision**: 문법 해석과 fixture는 `markdown-annotation-core`, React
트리 렌더링·주석 표시·HTML 적합성은 `markdown-annotation-react`로 분리한다. MA에는
공유 package의 API를 소비하기 위한 최소 변경과 회귀 테스트만 둔다. AW의 주석 영역
전환은 `features/worktree-workspace`에 국한된 화면 상태이므로 공유 package에 올리지
않는다.
