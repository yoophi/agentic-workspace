# UI Contract: CollapsibleResizablePanels

## Public Export

```ts
import {
  CollapsibleResizablePanels,
  type CollapsibleResizablePanelsProps,
  type CollapsibleResizablePanelsState,
} from "@yoophi/ui/components/collapsible-resizable-panels";
```

기존 package wildcard export를 사용하며 새로운 barrel export는 만들지 않는다.

## Props Contract

```ts
type PanelSide = "left" | "right";

interface CollapsiblePanelDefinition {
  id: string;
  title: React.ReactNode;
  content: React.ReactNode;
  collapsible?: boolean;
  contentClassName?: string;
  showTitle?: boolean;
  defaultSize?: string | number;
  minSize?: string | number;
  maxSize?: string | number;
}

interface CollapsibleResizablePanelsState {
  collapsedPanel: PanelSide | null;
  leftSize: number;
  rightSize: number;
}

interface CollapsibleResizablePanelsProps {
  leftPanel: CollapsiblePanelDefinition;
  rightPanel: CollapsiblePanelDefinition;
  collapsedSize?: string | number;
  className?: string;
  onStateChange?: (state: CollapsibleResizablePanelsState) => void;
}
```

구체 타입은 `react-resizable-panels` v4의 size 타입과 정렬하되 소비자가 기반 라이브러리의 imperative ref를 직접 다루지 않게 한다.

## Render Contract

- 루트는 수평 방향의 두 Panel과 그 사이 Separator를 렌더링한다.
- 펼침 상태의 각 패널은 위쪽 title button과 나머지 content region을 가진다.
- `showTitle`이 `false`인 패널은 title 영역 없이 content region이 전체 높이를 사용한다.
- `collapsible` 또는 `showTitle`이 `false`인 패널은 접기 조작을 제공하지 않는다.
- Panel A가 접히면 왼쪽 title rail만 남고 제목은 rail 우측 끝에 정렬된 후 −90도 회전한다.
- Panel B가 접히면 오른쪽 title rail만 남고 제목은 rail 우측 끝에 정렬된 후 −90도 회전한다.
- 접힌 패널의 content region은 렌더 트리에서 숨기고 title button은 rail 전체 크기를 차지한 채 계속 활성화한다.
- 접힌 title button의 제목 텍스트가 없는 빈 영역도 제목 텍스트와 동일한 활성화 동작을 제공한다.
- 반대쪽 마지막 열린 패널의 title button은 disabled다.
- 한 패널이 접힌 동안 Separator는 보이되 resize 입력을 받지 않는다.

## Behavior Contract

1. 두 패널은 모두 펼쳐진 상태로 시작한다.
2. Separator drag/keyboard resize는 두 패널이 모두 펼쳐진 동안에만 가능하다.
3. 제목 활성화는 해당 패널을 접고 접기 직전 너비를 해당 패널에 저장한다.
4. 접힌 rail의 제목 텍스트 또는 빈 공간 활성화는 해당 패널을 현재 공간에서 가능한 저장 너비로 동일하게 복원한다.
5. 다른 패널이 접힌 경우 마지막 열린 패널의 제목 활성화는 허용되지 않는다.
6. 상태 변경 완료 후 `onStateChange`는 collapse 상태와 확정 크기를 전달한다.
7. 앱 재시작 또는 remount 이후 크기 영속화는 제공하지 않는다.

## Accessibility Contract

- title control은 실제 `button` 요소이며 최소 상태에서는 rail 전체가 해당 button의 hit area다.
- title button은 콘텐츠 id를 `aria-controls`로 참조하고 `aria-expanded`로 상태를 전달한다.
- 마지막 열린 title button은 native `disabled`와 시각적 disabled 상태를 함께 갖는다.
- title button에는 명확한 focus-visible 표시가 있다.
- Separator의 `role="separator"`, orientation, keyboard resize 의미는 기반 라이브러리에서 유지한다.
- 회전은 시각 표현만 바꾸며 제목의 읽기 순서와 accessible name을 변경하지 않는다.

## Styling Contract

- 색상과 경계는 `background`, `foreground`, `border`, `muted-foreground`, `ring` 등 의미 기반 토큰을 사용한다.
- `className`은 소비자의 외부 레이아웃 크기 지정에 사용하며 내부 색상·타이포그래피 계약을 덮어쓰는 수단으로 사용하지 않는다.
- 두 패널 펼침, 접힘 title rail, separator hover/focus/disabled 상태를 data/ARIA 속성으로 식별 가능하게 한다.

## Error and Boundary Contract

- 중복되거나 빈 panel id는 개발 중 명확히 탐지할 수 있어야 한다.
- 저장 너비가 현재 min/max 또는 가용 공간과 충돌하면 유효 범위로 clamp한다.
- 빠른 반복 활성화에서도 둘 다 접힌 상태는 생성하지 않는다.
- 긴 제목은 rail 밖으로 넘치지 않으며 accessible name 전체는 유지한다.
- rail 상단, 중앙, 하단의 빈 공간은 모두 동일한 펼치기 동작을 제공한다.

## Non-contract

- 세 개 이상 패널
- 상하 배치
- localStorage 등 영구 크기 저장
- 앱 라우팅, Tauri 명령, 세션 상태와의 결합
