# UI Contract: Agent Run 탭·타일 워크스페이스

## View mode

- Toolbar는 `탭`과 `타일`을 단일 선택 그룹으로 표시하고 현재 값을 보조 기술에 전달한다.
- 초기값은 기존 동작과 같은 `tabs`다.
- 전환은 slot/layout을 수정하지 않고 projection만 바꾼다.
- `tabs`에서는 `focusedPanelId` panel만 보이고 나머지는 mounted-hidden 상태를 유지한다.
- `tiles`에서는 layout tree의 모든 leaf가 표시되고 `focusedPanelId` 타일은 색상 외 focus ring/label로 구분한다.

## Panel identity and lifecycle

- `AgentRunPanel` key는 항상 `panelId`이며 view mode 또는 tree 위치로 만들지 않는다.
- Main panel은 항상 존재하고 close command를 노출하지 않는다.
- extra panel의 기존 running close confirmation과 cancel behavior를 유지한다.
- 각 panel의 run event, permission request, prompt draft, queue, timeline scroll, minimap state는 view 전환과 resize 중 유지된다.

## Open-adjacent commands

- tile header와 workspace command menu는 `오른쪽에 새 타일 열기`, `아래에 새 타일 열기`를 제공한다.
- command target은 command를 연 타일이며, global shortcut/menu에서는 `focusedPanelId`다.
- 명령은 새 extra slot과 leaf를 하나의 reducer transaction으로 추가한다.
- 성공하면 view mode는 `tiles`, 새 panel이 focused target이 된다.
- container geometry, panel count 또는 depth 제한을 위반하면 state를 변경하지 않고 이유를 status message로 알린다.
- 기존 탭의 `+`는 extra slot을 추가하되 layout에는 focused leaf 오른쪽 분할로 등록한다. 탭 모드를 자동 전환하지 않는다.

## Tile rendering

- split orientation은 `horizontal`이면 좌우, `vertical`이면 상하로 렌더링한다.
- separator는 pointer drag와 keyboard step을 지원하고 두 인접 영역을 설명하는 accessible name을 가진다.
- tile header는 title, running indicator, exchange badge, focus 상태, peer message command, split command, close command를 제공한다.
- 타일 콘텐츠가 좁아져도 prompt send/cancel, permission request, timeline scroll의 핵심 조작은 접근 가능해야 한다.

## Focus and keyboard

- tile body 또는 header와 상호작용하면 해당 `panelId`가 focused가 된다.
- `Ctrl+Option+Arrow`는 spatial neighbor tile로 focus를 이동한다.
- `Ctrl+Option+T`는 tab/tile view를 전환한다.
- command menu에는 shortcut을 표시하되 플랫폼 충돌이 확인되면 최종 구현에서 조정할 수 있다.
- 닫힌 focused tile의 sibling subtree 첫 leaf가 우선 focus를 받고, 없으면 tree order상 이전/다음, 마지막으로 Main을 사용한다.

## External prompt routing

- workspace annotation/SDD prompt는 항상 `focusedPanelId`로 전달한다.
- tab mode의 background target이나 tile mode의 다른 target으로 exchange가 도착해도 focus/view mode를 바꾸지 않는다.
- `send`, `queue`, `draft` 의미는 기존 `AgentRunPanel` contract를 그대로 따른다.

## Exchange UI

- peer message dialog는 현재 workspace snapshot의 closing이 아닌 다른 panel을 표시한다.
- target label은 title, running/idle 상태를 함께 보여주고 주소는 panel ID로 유지한다.
- 사용자는 delivery를 선택하고 16KiB 상한 전 남은 크기를 확인한다.
- source와 target tile은 exchange status를 pending/accepted/delivered/rejected/failed/cancelled로 표시한다.
- error는 message를 잃지 않으며 target 또는 delivery를 바꿔 새 request ID로 재시도할 수 있다.

## Storybook states

- molecule: view toggle, tile header idle/running/focused/error badge
- organism: one tile, two horizontal, nested horizontal+vertical, eight-panel limit, narrow container, running close confirmation, exchange dialog/status
- page: full Worktree Session tab mode and tile collaboration mode
