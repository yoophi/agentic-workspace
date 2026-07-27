# Research: Agent Run 탭·타일 워크스페이스

## 1. 타일 레이아웃 표현

**Decision**: 패널 leaf와 방향성 split으로 구성된 이진 분할 트리를 사용한다.

**Rationale**: "현재 타일 오른쪽/아래에 열기"를 대상 leaf의 국소 변환으로 정확히 표현할 수 있다. 닫기는 부모 split을 형제 노드로 축약하면 공간 회수가 결정적이며, 각 split ratio를 기존 `react-resizable-panels`에 직접 대응할 수 있다.

**Alternatives considered**:

- 자동 CSS grid: 동시 관찰에는 단순하지만 사용자가 요청한 인접 방향과 중첩 관계를 보존하기 어렵다.
- 자유형 rectangle 좌표: 유연하지만 충돌·빈 공간·resize 정규화가 복잡하고 키보드 접근성이 나쁘다.
- panel 배열 + row/column index: 2차원 균등 격자에는 적합하지만 중첩 분할을 표현하지 못한다.

## 2. 탭/타일 전환 시 panel state 보존

**Decision**: 보기 모드는 렌더 projection만 바꾸며 slot과 layout은 항상 유지한다. 각 `AgentRunPanel`은 panel ID를 key로 유지하고, 탭 모드에서는 비활성 panel을 숨기되 unmount하지 않는다.

**Rationale**: 현재 구현도 모든 panel을 mount한 상태에서 비활성 panel만 `hidden` 처리한다. 동일 identity를 유지하면 run event subscription, 입력 초안, 스크롤, 미니맵, 권한 요청을 재구성하지 않고 전환할 수 있다.

**Alternatives considered**:

- 보기 전환마다 panel remount: 구현은 단순하지만 로컬 state와 ref가 손실되고 active run event 연결 회귀 위험이 크다.
- panel state 전체를 상위 reducer로 끌어올림: 장기적으로 가능하지만 이번 기능에 불필요한 대규모 리팩터링이다.

## 3. 에이전트 런 간 통신 경계

**Decision**: backend application service와 window-scoped workspace registry를 통신 권한 경계로 사용한다. UI component 간 직접 callback이나 전역 event bus만으로 통신하지 않는다.

**Rationale**: agent가 MCP에서 메시지를 보낼 때 source run 소유권을 backend가 검증해야 한다. 동일 창 label과 canonical worktree path를 함께 확인하면 다른 창·worktree의 run을 추측한 ID로 호출하는 것을 차단할 수 있다.

**Alternatives considered**:

- frontend-only event bus: 사용자 UI에는 충분하지만 MCP agent source의 소유권과 target run 유효성을 신뢰할 수 없다.
- `acp-agent-core`의 `SessionRegistry`에 panel/worktree 개념 추가: 공유 ACP core에 AW UI 개념이 유입되고 Hushline 등 다른 consumer에 불필요한 결합을 만든다.
- agent subprocess끼리 직접 연결: ACP 세션 격리를 깨고 lifecycle, permission, audit 경계를 우회한다.

## 4. 메시지 전달 transport

**Decision**: application service가 검증된 exchange를 target window event로 emit하고 frontend가 target panel의 기존 `AgentPromptRequest`에 적용한 뒤 acknowledge한다.

**Rationale**: 현재 `AgentRunPanel`은 이미 `send`, `queue`, `draft` delivery를 처리한다. 이 경로를 재사용하면 active/idle 상태, queue, textarea draft 동작을 중복 구현하지 않고 panel identity에 정확히 전달할 수 있다. Backend는 accepted와 final delivery 상태를 분리해 race를 기록한다.

**Alternatives considered**:

- backend에서 target `SessionHandle`에 직접 prompt: 즉시 send는 가능하지만 draft UI와 frontend queue를 표현하지 못하고 panel-level 상태를 건너뛴다.
- exchange별 별도 채팅 저장소: v1의 ephemeral 협업에 과도하며 영속·재생 정책을 새로 정해야 한다.

## 5. MCP tool surface

**Decision**: `list_peer_agents`, `send_message_to_agent`, `get_agent_exchange_status` 세 도구를 기존 AW MCP server에 추가한다.

**Rationale**: agent가 안전한 target을 먼저 조회하고, 안정적인 panel ID/current run ID로 전송하며, 비동기 frontend acknowledgement 결과를 조회할 수 있다. `runId`는 agent에게 주입된 `AW_MCP_RUN_ID`와 반드시 일치하는 active source여야 한다.

**Alternatives considered**:

- target title을 직접 입력: title은 중복·변경 가능하므로 주소로 부적합하다.
- send 도구 하나만 제공: target discovery와 asynchronous final status를 자연어 추측에 의존하게 된다.
- 모든 peer 대화를 자동 relay: 비용·루프·권한 위험이 크고 사용자 의도 없이 에이전트들이 상호 호출할 수 있다.

## 6. Exchange idempotency와 lifecycle

**Decision**: client-generated UUID를 request ID로 사용하며 상태는 `pending → accepted → delivered | rejected | failed | cancelled` 단방향 전이만 허용한다. window별 최근 500개를 메모리에 유지한다.

**Rationale**: Tauri/MCP retry와 event fallback 중복에도 target panel에 한 번만 적용할 수 있다. terminal state 역행을 금지하면 늦은 acknowledgement가 상태를 덮지 않는다.

**Alternatives considered**:

- 메시지 텍스트 hash로 중복 제거: 같은 지시를 의도적으로 반복하는 정상 사용을 막는다.
- 무제한 기록: 장시간 세션에서 메모리 증가가 제어되지 않는다.
- 영구 JSON 기록: restart 복원과 개인정보 보존 정책이 필요하므로 v1 범위를 넘는다.

## 7. 타일 제한과 접근성

**Decision**: panel 8개, split depth 4를 v1 검증 범위로 두고 실제 container geometry가 최소 타일 크기를 만족하지 못하면 분할을 거부한다. toolbar, tile header, split separator는 roving focus와 명시적 accessible name을 제공한다.

**Rationale**: AgentRunPanel은 timeline과 prompt를 함께 보여야 하므로 지나치게 작은 타일은 기능을 사용할 수 없다. 고정 수치만 보지 않고 실제 방향별 공간을 확인해야 outer workspace 폭 변화에도 안전하다.

**Alternatives considered**:

- 무제한 분할: 보이지 않는 panel과 과도한 event/render 비용을 만든다.
- 자동으로 기존 타일 닫기: 사용자 run을 파괴한다.
- 좁아지면 자동 tab 전환: 예측 불가능하게 현재 비교 화면을 바꾸므로 안내 후 사용자 선택을 우선한다.
