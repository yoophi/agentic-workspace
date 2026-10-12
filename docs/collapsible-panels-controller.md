# 공유 패널 외부 제어와 내용 보존

`@yoophi/ui/components/collapsible-resizable-panels`는 기존 제목 버튼·접힌 rail과 동일한 경로를 외부 제어에도 제공한다. Box의 문서/ACP 배치에서 재사용하며 ACP 전송이나 세션 수명에는 관여하지 않는다.

## 선택 API

| 항목 | 계약 |
| --- | --- |
| `CollapsiblePanelDefinition.keepMounted` | 기본 false. true이면 내용 wrapper·DOM·React component를 접어도 유지한다. 접힌 wrapper는 `hidden`, `inert`, inline `display: none`으로 숨기며 focus를 받지 않는다. |
| `defaultCollapsedPanel` | 기본 null. 최초 유효한 측정 레이아웃을 저장한 뒤 지정한 쪽을 접는다. zero-width/숨겨진 그룹은 측정을 기다린다. 이후 prop 변경으로 초기 상태를 재적용하지 않는다. |
| `controllerRef` | 선택 React ref. `collapse(side)`, `expand(side)`, `toggle(side)`, `getState()`를 제공하며 `side`는 `left` 또는 `right`다. |
| `getState()` / `onStateChange` | 현재 접힌 쪽과 마지막 열린 너비(백분율)를 제공한다. 접힌 rail의 실제 폭을 열린 너비로 덮어쓰지 않는다. |

`collapsible: false` 또는 `showTitle: false`인 패널에는 외부/초기 접기도 적용하지 않는다. 한 쪽이 접혀 있을 때 나머지 쪽을 접는 요청은 무시한다. 반복 collapse/expand는 멱등이며 측정 전에 호출한 collapse는 무시한다. 최초 측정 전 `getState()`는 기존 기본 상태를 제공하므로 실제 px 복원 폭으로 사용하지 않는다.

내용 수명은 `keepMounted`로 결정한다. 기존 호출자는 여전히 접을 때 내용을 언마운트한다. 보존을 선택하면 내용의 effect·스트림 구독도 살아 있으므로 비용이 큰 작업의 중지는 소비자가 별도로 관리한다. 토글 자체로 ACP cancel/close를 호출하지 않는다.

## 사용 예

```tsx
const panels = useRef<CollapsibleResizablePanelsHandle>(null);

<button onClick={() => panels.current?.toggle("right")}>채팅</button>
<CollapsibleResizablePanels
  controllerRef={panels}
  defaultCollapsedPanel="right"
  leftPanel={{
    id: "document", title: "문서", content: <Editor />,
    collapsible: false, showTitle: false, keepMounted: true,
  }}
  rightPanel={{
    id: "chat", title: "채팅", content: <Chat />,
    keepMounted: true, defaultSize: "320px", minSize: "160px",
  }}
/>
```

그룹과 내용에 접기 상태 기반의 key를 주거나 그룹 자체를 조건부 제거하면 보존 계약을 사용할 수 없다.

## 너비와 검증

실제 `react-resizable-panels`의 측정 결과를 저장한다. 숫자는 px이며 `"320px"`와 같다. 백분율과 px default 및 min/max 해석을 공용 component가 복제하지 않고 primitive에 맡긴다. 펼칠 때 저장된 백분율로 resize를 요청하며 현재 컨테이너와 양쪽 min/max 경계가 최종 폭을 제한한다. 좁은 창에서 원래 px 폭을 그대로 유지한다는 계약은 없다.

GE Molecules Storybook의 `PersistentPanelsPercentage`, `PersistentPanelsPixels`, `PersistentPanelsNumericPixels`, `PersistentPanelsConstrained`는 외부 제어·초기 접기·두 입력 초안·container resize·제약 변경을 제공한다.

- `pnpm --filter @yoophi/ui test`: 기존 순수 상태 테스트와 DOM/component 테스트. DOM 테스트의 resize primitive는 mock이므로 실제 크기 증거가 아니다.
- `pnpm --filter @yoophi/ui test:browser`: GE Storybook 빌드 후 실제 Chromium primitive와 pointer/keyboard resize, 측정 폭 복원, DOM/React 입력/undo, hidden/inert focus, 경계 및 마지막 열린 패널 보호를 검증한다. Playwright Chromium 설치가 필요하다.
- `pnpm --filter @yoophi/ui check-types`, `pnpm --filter @yoophi/git-explorer build`: 공유 패키지와 기존 소비자 타입/번들 검증.

이 검증은 공유 웹 컴포넌트의 근거이며 Box의 실제 native 앱 또는 ACP 응답 공존을 증명하지 않는다. Box vendor pin/facade/inline 배치는 gh-46, 실제 ACP/native 최종 gate는 gh-47에서 별도로 수행한다.

## gh-45 검증 결과 (2026-10-13)

별도 `box-md-ui-panels` 브랜치에서 기존 상태 9개 + component/DOM 8개, 실제 Chromium 9개가 통과했다. UI 타입 검사, GE 타입/프로덕션 빌드, GE Storybook 빌드도 통과했다. Chromium은 기존 소비자의 기본 unmount와 `showTitle: false` 동작, 숫자/문자열 px·백분율·max 경계 초기 복원, 좁은 컨테이너 min/max, pointer/keyboard resize, 문서 및 접혔던 채팅의 undo를 포함한다.

DOM mock 결과와 실제 브라우저 결과는 분리해서 확인했다. 사용자 AW primary checkout과 기존 `box-md-http-client` 브랜치는 변경하지 않았다.
