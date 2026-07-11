# Implementation Plan: 접기 및 크기 조절이 가능한 패널

**Branch**: `028` | **Date**: 2026-07-11 | **Spec**: [spec.md](./spec.md)

**Input**: `specs/028-collapsible-resizable-panel/spec.md`와 사용자 기술 지침: `react-resizable-panels` 기반, `packages/ui` 공용 컴포넌트, Storybook 확인 가능

## Summary

좌우 두 패널을 세로 구분선으로 조절하고 각 제목으로 접고 펼칠 수 있는 `CollapsibleResizablePanels` 공용 컴포넌트를 `packages/ui`에 추가한다. 이미 설치된 `react-resizable-panels` v4와 shadcn `Resizable` 래퍼를 기반으로 하며, 패널별 마지막 너비 보존, 최소 한 패널 펼침 보장, 접힌 rail 전체의 활성화, 회전 제목의 우측 끝 정렬, 접힌 동안 구분선 비활성화를 상위 상태 전이와 UI 계약으로 관리한다. Git Explorer Storybook의 Atomic Design/Molecules 카탈로그에서 주요 상태와 rail 위치별 상호작용을 확인하고 공용 패키지 단위 테스트와 소비 앱 검증을 수행한다.

## Technical Context

**Language/Version**: TypeScript 5.8, React 19.1

**Primary Dependencies**: `react-resizable-panels` 4.11.2, 기존 `@yoophi/ui/components/resizable`, Tailwind CSS 4, shadcn radix-nova 구성

**Storage**: 영구 저장 없음. 컴포넌트 수명 동안 React 상태와 ref에 패널별 마지막 펼침 너비만 보존

**Testing**: Vitest 4.1.9 순수 상태 전이 단위 테스트, TypeScript 타입 검사, Git Explorer Storybook 10.4.6 상호작용 테스트(rail 상·중·하단 활성화와 제목 정렬 포함)·a11y 확인 및 정적 빌드

**Target Platform**: React 19을 사용하는 데스크톱 웹뷰 및 현대 데스크톱 브라우저

**Project Type**: pnpm/Turbo 모노레포의 공용 React UI 라이브러리와 Storybook 소비 앱

**Performance Goals**: 드래그와 접기·펼치기 상태 전환이 일반적인 60Hz 화면에서 끊김 없이 보이고, 포인터 이동마다 불필요한 앱 수준 렌더링이나 영구 저장을 수행하지 않음

**Constraints**: 좌우 두 패널만 지원, 최소 한 패널은 항상 펼침, 한 패널이 접히면 구분선 비활성, 최소 상태 rail 전체가 하나의 title button으로 동작, 회전 제목은 양쪽 rail의 우측 끝 정렬, 제목은 키보드와 보조 기술 상태 제공, raw 색상 대신 의미 기반 디자인 토큰 사용

**Scale/Scope**: 공용 컴포넌트 1개, 순수 상태 모듈 1개, 단위 테스트 1개, Git Explorer Storybook 스토리 모음 1개; 기존 앱 화면 교체는 범위 밖

## Constitution Check

*GATE: Phase 0 전과 Phase 1 설계 후 모두 통과.*

- **Monorepo Boundary First — PASS**: 재사용 구현은 `packages/ui/src/components`에 두고 Storybook 소비 예시는 `apps/git-explorer/src/stories`에 둔다. 앱 간 직접 import가 없다.
- **Feature-Sliced Frontend Architecture — PASS**: shadcn 기반 공용 UI는 `packages/ui/src/components`에 유지하고, 소비 앱에는 제품 기능 코드가 아닌 Storybook 등록만 추가한다.
- **Hexagonal Tauri Backend Architecture — N/A**: Tauri 명령, 저장소, 파일 시스템 변경이 없다.
- **Shared Core Before Shared UI — PASS**: 패널 상태 전이는 UI와 분리된 순수 모듈로 만들고, 상위 UI는 앱 셸·라우팅·Tauri API에 의존하지 않는다. 공통 fixture와 단위 테스트로 공유 타당성을 검증한다.
- **Atomic Cross-App Verification — PASS**: `@yoophi/ui`의 `check-types`와 `test`, 현재 유일한 소비 앱 `@yoophi/git-explorer`의 `check-types`와 Storybook 정적 빌드를 계획한다.
- **Documentation and Storybook — PASS**: Git Explorer의 `Atomic Design/Molecules`에 두 패널 펼침, 좌·우 접힘, 긴 제목·콘텐츠 상태를 등록한다. 별도 `docs/*.md` 사용자 문서는 필요하지 않으며 설계 산출물은 `specs/028-*`에 둔다.
- **Testing and Safety — PASS**: 패널별 크기 보존, 최소 한 패널 보장, 구분선 활성 여부를 순수 상태 전이 테스트로 검증한다. 파일·세션·권한 데이터는 다루지 않는다.

### Post-Design Re-check

`data-model.md`가 순수 상태와 전이를 분리하고, `contracts/ui-component.md`가 앱 비의존 공개 API와 접근성 계약을 제한하며, `quickstart.md`가 공용 패키지 및 유일 소비 앱 검증을 포함하므로 모든 적용 게이트가 계속 PASS다.

**Agent context update**: 이 저장소의 `.specify/scripts`에는 `update-agent-context.sh` 또는 동등한 실행 스크립트가 없으므로 생성형 agent context 변경 없이 단계를 건너뛰었다.

## Project Structure

### Documentation (this feature)

```text
specs/028-collapsible-resizable-panel/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── ui-component.md
└── tasks.md                 # /speckit-tasks에서 생성
```

### Source Code (repository root)

```text
packages/ui/
├── package.json
└── src/components/
    ├── resizable.tsx
    ├── collapsible-resizable-panels.tsx
    ├── collapsible-resizable-panels-state.ts
    └── collapsible-resizable-panels-state.test.ts

apps/git-explorer/src/stories/
└── molecules.stories.tsx
```

**Structure Decision**: 기존 shadcn `resizable.tsx`는 범용 하위 프리미티브로 유지한다. 새 상위 컴포넌트와 순수 상태 전이는 `packages/ui`에 추가하고 package export wildcard를 그대로 사용한다. 별도 Storybook 인프라를 중복 생성하지 않고 이미 공용 UI를 등록하는 Git Explorer의 Molecules 스토리에 상호작용 가능한 사례를 추가한다.

## Implementation Phases

### Phase 0 — 기반 계약 확정

1. `react-resizable-panels` v4의 `Group`, `Panel`, `Separator`, imperative panel ref와 disabled 동작을 기준으로 구현 계약을 고정한다.
2. 패널별 마지막 펼침 너비와 접힘 상태를 순수 상태 전이로 정의한다.
3. 공개 props와 콜백, 접근성 및 비범위를 `contracts/ui-component.md`에 고정한다.

### Phase 1 — 공용 컴포넌트

1. 상태 전이 모듈과 경계·불변식 단위 테스트를 먼저 추가한다.
2. 기존 `ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle`을 조합한 `CollapsibleResizablePanels`를 구현한다.
3. 최소 상태 rail 전체를 title button의 hit area로 만들고 회전 제목을 rail 우측 끝에 정렬하며, disabled separator에 의미 기반 토큰과 focus-visible 상태를 적용한다.
4. `packages/ui/package.json`에 테스트 스크립트와 필요한 개발 의존성을 추가한다.

### Phase 2 — Storybook 및 검증

1. Git Explorer Molecules 스토리에 양쪽 펼침, Panel A 접힘, Panel B 접힘, 긴 제목·콘텐츠 사례를 추가한다.
2. 마우스·키보드 접기/펼치기, 최소 상태 rail의 상·중·하단 활성화, 양쪽 회전 제목의 우측 끝 정렬, 마지막 열린 제목 비활성, 크기 복원, 접힌 상태 구분선 비활성을 확인한다.
3. 패키지 테스트·타입 검사와 소비 앱 타입 검사·Storybook 정적 빌드를 실행한다.

## Completion Criteria

- 공개 UI 계약의 모든 상태 전이, rail 전체 활성화, 제목 우측 끝 정렬 및 접근성 조건이 구현되고 테스트된다.
- Storybook에서 명세의 대표 상태를 독립적으로 재현할 수 있다.
- `@yoophi/ui` 테스트·타입 검사와 `@yoophi/git-explorer` 타입 검사·Storybook 빌드가 통과한다.
- 기존 `Resizable` 소비 코드의 타입과 동작을 변경하지 않는다.

## Complexity Tracking

헌법 위반이 없어 기록할 항목이 없다.
