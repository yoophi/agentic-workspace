# Research: Agent Run 히스토리 미니맵

## R1. 통합 경계와 상태 소유권

- **Decision**: 미니맵을 각 `AgentRunPanel`의 상단 히스토리 패널 안에 배치하고 표시 여부는 패널 로컬 상태로 관리한다.
- **Rationale**: `WorktreeAgentRunArea`는 각 panel slot을 마운트한 채 비활성 panel만 숨기므로 로컬 상태만으로 표시 여부와 스크롤 위치가 패널별로 보존된다. 하단 prompt composer나 우측 workspace는 미니맵 탐색 대상이 아니다.
- **Alternatives considered**: `ProjectWorktreeSessionPage`에서 전체 agent 영역을 분할하면 prompt composer까지 잘못 포함한다. slot reducer나 전역 상태는 세션 중 UI 표시 상태에 비해 복잡하고 영속화 요구도 없다.

## R2. 미니맵 콘텐츠 표현

- **Decision**: 정규화된 `TimelineItem[]`에서 `user/message`, `assistant/message`만 뽑아 발화자, 순서, 공백이 정규화된 짧은 요약, 제한된 상대 크기를 가진 `MinimapEntry`로 투영한다.
- **Rationale**: `TimelineItem`은 raw provider 이벤트 차이를 이미 흡수하며 연속 agent 출력 병합도 반영한다. 의미 기반 축약은 사용자가 원하는 대화 흐름을 보존하면서 무거운 Markdown, Mermaid, 도구 UI를 중복 렌더링하지 않는다.
- **Alternatives considered**: 실제 타임라인 DOM 축소/복제는 가상화 때문에 대부분의 항목이 DOM에 없고, 비싼 렌더링과 중복된 대화형 요소를 만든다. 모든 이벤트를 표시하면 prompt/응답 탐색 신호가 약해진다.

## R3. 좌표계와 스크롤 동기화

- **Decision**: 기존 `VirtualizedRunTimeline`의 측정/추정 item layout, timeline 시작 offset, 전체 timeline 높이, 보이는 시작/끝을 하나의 layout snapshot으로 공유하고 미니맵과 히스토리 양쪽이 이를 사용한다.
- **Rationale**: history scroll element에는 고정 header, 설정, 필터 및 queue 영역도 포함되므로 단순 `scrollTop / scrollHeight`는 타임라인 위치와 어긋난다. 항목 높이도 Markdown 내용에 따라 달라 SC-003의 5% 정합성을 위해 동일 좌표계가 필요하다.
- **Alternatives considered**: 균등 간격 항목과 전체 scroller 비율은 긴/짧은 메시지가 섞일 때 오차가 누적된다. 별도 observer와 독립 auto-follow 정책은 기존 listener와 경쟁해 jitter를 만들 수 있다.

## R4. 가상화와 스트리밍 정책

- **Decision**: 기존 96px 초기 추정, 실제 높이 측정, 6항목 overscan 및 하단 48px 이내 자동 추적 정책을 유지한다. 미니맵은 snapshot을 읽고 clamp된 목표만 기존 scroller에 전달한다.
- **Rationale**: 현재 정책은 사용자가 과거를 보면 새 출력이 위치를 빼앗지 않고, 최신을 보면 계속 따라가는 명세 요구를 이미 충족한다. 측정값이 바뀌면 snapshot을 갱신해 indicator를 보정한다.
- **Alternatives considered**: 미니맵 전용 stick-to-bottom 상태는 두 정책이 서로 덮어쓸 위험이 있다. 모든 500항목을 실제 렌더링하면 성능 목표와 현재 가상화 이점을 잃는다.

## R5. 이벤트 필터와 미니맵의 관계

- **Decision**: 미니맵 콘텐츠는 필터되지 않은 전체 `items`의 대화 흐름을 안정적으로 유지한다. 사용자가 Tool/Lifecycle 등 대화 항목을 숨기는 필터에서 미니맵 탐색을 시작하면 `All` 필터로 전환하고, 레이아웃 갱신 후 보류된 normalized target을 적용한다.
- **Rationale**: 미니맵이 현재 필터를 따르면 핵심 prompt/agent 흐름이 빈 상태가 되어 FR-002를 위반한다. 반대로 숨겨진 항목을 그대로 탐색하면 실제 히스토리에 목표가 없어 정확한 anchoring을 보장할 수 없다.
- **Alternatives considered**: 필터 상태에서 미니맵을 비활성화하면 기능 가용성이 불필요하게 떨어진다. 필터된 타임라인 비율에 대화 항목을 억지로 대응시키면 사용자에게 잘못된 위치를 보여준다.

## R6. 포인터와 키보드 접근성

- **Decision**: viewport indicator를 세로 slider 의미로 제공하고 pointer capture drag, clamp, ArrowUp/Down, PageUp/Down, Home/End를 지원한다. 토글은 lucide icon과 tooltip, 명시적 label 및 pressed/expanded 상태를 갖는다.
- **Rationale**: pointer capture는 rail 밖으로 이동해도 드래그를 안정적으로 끝낼 수 있고, slider 계약은 포인터를 쓰지 못하는 사용자에게 동일한 시작/중간/끝 탐색을 제공한다.
- **Alternatives considered**: drag-only UI는 FR-014와 FR-015를 충족하지 못한다. 텍스트 버튼은 제한된 work surface에서 불필요한 공간을 차지한다.

## R7. 레이아웃 방식

- **Decision**: 기존 history를 `minmax(0, 1fr)` 왼쪽 열로 유지하고 우측에 고정·제한 폭의 미니맵 rail을 둔다. 별도 resize handle은 추가하지 않으며 숨김 시 scroll node를 유지한 채 rail 열만 제거한다.
- **Rationale**: 사용자는 미니맵 표시 전환과 viewport drag를 요구했지만 폭 조절은 요구하지 않았다. 고정 rail은 최소 360px agent pane에서도 안정적인 크기를 제공하고 토글 시 `scrollTop` 보존이 쉽다.
- **Alternatives considered**: 중첩 resizable panel은 resize handle과 드래그 indicator의 역할을 혼동시키고 상태 복잡도를 늘린다. overlay는 히스토리 콘텐츠를 가릴 수 있다.

## R8. 테스트 전략

- **Decision**: 투영, 요약, 상대 크기, layout 변환, clamp, 필터 전환 후 pending seek, 키보드 이동은 Vitest 순수 모델 테스트로 검증한다. 실제 pointer capture, scroll sync, 토글, 스트리밍, 좁은 레이아웃은 Storybook addon-a11y와 브라우저에서 검증한다.
- **Rationale**: 현재 Vitest 환경은 Node 중심이고 jsdom/Playwright 구성이 없어 소스 문자열 테스트만으로 drag 동작을 보장할 수 없다. 순수 계산은 자동화하고 실제 WebView에 가까운 브라우저 상호작용은 실행 가능한 Storybook 상태로 검증하는 조합이 현실적이다.
- **Alternatives considered**: 새 브라우저 테스트 스택 도입은 이 기능의 범위를 넓힌다. SSR/source assertion만으로는 geometry와 pointer 동작을 검증할 수 없다.

## R9. 백엔드와 공유 범위

- **Decision**: Tauri/Rust, persistence, repository, `packages/*`, `crates/*`를 변경하지 않는다. 내부 UI 계약만 문서화한다.
- **Rationale**: 필요한 타임라인과 session/panel 경계가 이미 프론트엔드 메모리에 있고 소비 앱도 AW 하나뿐이다. app-local 순수 모델과 UI가 헌법의 공유 기준에 맞는다.
- **Alternatives considered**: 표시 설정 영속화는 명세에서 제외되었고, cross-app package 추출은 두 번째 소비자가 없어 시기상조다.
