# Research: 접기 및 크기 조절이 가능한 패널

## Decision 1: 기반 라이브러리

- **Decision**: 요청의 `react-resizable-pane`은 프로젝트에 이미 설치된 `react-resizable-panels` v4.11.2로 해석하고 이를 사용한다.
- **Rationale**: `packages/ui/package.json`과 lockfile에 이미 존재하며, shadcn 공식 `Resizable`도 이 라이브러리를 기반으로 한다. v4는 `Group`, `Panel`, `Separator`, `disabled`, `collapsedSize`, imperative panel ref를 제공해 요구 동작을 지원한다.
- **Alternatives considered**: 단수형 이름의 별도 패키지 추가는 현재 저장소 구성과 shadcn 프리미티브를 중복시키므로 제외했다. CSS pointer drag를 직접 구현하는 방안은 키보드·ARIA separator 동작을 다시 구현해야 하므로 제외했다.
- **Sources**: [shadcn Resizable](https://ui.shadcn.com/docs/components/radix/resizable), [react-resizable-panels 공식 저장소/API](https://github.com/bvaughn/react-resizable-panels)

## Decision 2: 기존 프리미티브 위 상위 컴포넌트

- **Decision**: `resizable.tsx`를 수정해 특수 동작을 섞지 않고, 이를 조합하는 `CollapsibleResizablePanels`를 별도 export로 추가한다.
- **Rationale**: 기존 Git Explorer 화면은 2개 및 3개 일반 resizable layout을 사용한다. 특수한 “두 패널·최소 한 패널 펼침” 규칙을 기존 프리미티브에 넣으면 기존 소비자 계약을 훼손한다.
- **Alternatives considered**: 기존 `ResizablePanel`에 제목·접힘 props를 추가하는 방식은 기본 primitive의 책임과 API를 비대하게 만들어 제외했다.

## Decision 3: 상태 관리와 크기 복원

- **Decision**: `leftCollapsed`, `rightCollapsed`, 패널별 `lastExpandedSize`를 순수 상태로 관리하고, resize 완료 시 양쪽 크기를 갱신한다. 제목 접기 직전에 해당 패널 크기를 저장하고 펼칠 때 현재 제약 내에서 imperative resize/expand로 복원한다.
- **Rationale**: 패널별 크기 보존과 “둘 다 접힘 금지”를 하나의 reducer 전이에서 검증할 수 있다. `onLayoutChanged`는 포인터를 놓은 뒤 호출되어 매 이동마다 상위 상태를 갱신하지 않는 데 적합하다.
- **Alternatives considered**: 라이브러리 내부 복원만 의존하면 패널별 저장값과 소비자 콜백 계약을 명시적으로 시험하기 어렵다. localStorage는 명세의 비범위다.

## Decision 4: 접힌 상태와 구분선

- **Decision**: 한 패널이 접히면 Group 또는 Separator의 resize 동작을 비활성화하고, 접힌 패널에는 고정 최소 너비의 세로 title rail만 표시한다. rail 전체를 하나의 title button 영역으로 사용하고 Panel A/B의 회전 제목은 각각 rail 우측 끝에 배치한다. 두 패널이 모두 펼쳐져야 구분선이 다시 활성화된다.
- **Rationale**: 명세가 접힌 상태에서 drag 불가와 rail 어느 지점에서든 복구 가능함을 요구한다. rail 전체를 실제 button으로 만들면 별도 클릭 위임 없이 포인터·키보드 의미가 일치하고, 제목만 내부 정렬하면 큰 hit area와 작은 시각 라벨을 동시에 유지할 수 있다.
- **Alternatives considered**: separator를 DOM에서 제거하면 레이아웃과 focus 순서가 변하므로 disabled 상태 유지가 더 안정적이다.

## Decision 5: 접근성

- **Decision**: 펼친 제목과 접힌 rail 전체는 동일한 실제 button 의미를 갖는다. 회전 제목은 button 내부의 시각 라벨이며 별도 조작 요소가 아니다. 마지막으로 열린 패널의 제목 button은 disabled 처리하며 접힘 상태는 `aria-expanded`, 연결 콘텐츠는 `aria-controls`로 전달한다. Separator의 라이브러리 제공 WAI-ARIA와 키보드 조절을 유지한다.
- **Rationale**: 포인터와 키보드 동등성, 현재 상태, 클릭 불가 상태를 보조 기술에 명확히 전달한다.
- **Alternatives considered**: 클릭 가능한 heading/div는 별도 키보드와 ARIA 구현이 필요해 제외했다.

## Decision 6: Storybook 위치와 분류

- **Decision**: Git Explorer의 `apps/git-explorer/src/stories/molecules.stories.tsx`에 `Atomic Design/Molecules` 사례로 등록한다.
- **Rationale**: `packages/ui`에는 독립 Storybook 설정이 없고, Git Explorer Storybook은 이미 `@yoophi/ui`의 `Resizable` 등 공용 조합 컴포넌트를 카탈로그화한다. 새 Storybook 인프라 없이 사용자 요구를 충족하고 실제 소비 경로도 검증한다.
- **Alternatives considered**: `packages/ui` 전용 Storybook 신설은 설정과 의존성을 중복시키며 이번 컴포넌트 범위를 크게 늘려 제외했다. Agentic Workbench Storybook은 현재 `@yoophi/ui` 소비자가 아니다.

## Decision 7: 검증 범위

- **Decision**: 순수 reducer 단위 테스트, `@yoophi/ui` 타입 검사, `@yoophi/git-explorer` 타입 검사, Storybook 정적 빌드와 a11y 확인을 수행한다.
- **Rationale**: `@yoophi/ui`의 현재 유일한 외부 소비자는 Git Explorer다. 공용 패키지 변경과 실제 소비자 검증을 함께 수행하면 헌법의 Atomic Cross-App Verification을 충족한다.
- **Alternatives considered**: 모든 앱 전체 빌드는 직접 소비 관계가 없어 필수 범위에서 제외하되 루트 회귀 검증이 필요하면 추가 실행할 수 있다.

## Resolved Unknowns

모든 기술 선택과 통합 지점이 결정되었으며 `NEEDS CLARIFICATION` 항목은 없다.
