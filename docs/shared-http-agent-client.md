# 공유 HTTP 에이전트 채팅 클라이언트

box.md gh-3의 서브 task gh-23에서 `packages/workbench-agent-client`를 구현했다. 이 Module은 AW의 기존 `workbench-client` 연결·HTTP call·WebSocket 이벤트 클라이언트를 조립하고 작업대, 세션, 대화와 권한 상태를 관리한다. 자체 ACP framing, Tauri 메시지 중계, fetch 재시도 또는 WebSocket 재연결 코드를 만들지 않는다.

## 소비 Interface

`createAgentChatClient({ workspace, connectionProvider })`로 창/작업 디렉토리별 객체를 생성한다. connectionProvider는 native shell이 발급한 짧은 창 연결 정보만 반환한다. credential은 기존 연결 Module의 메모리에 보관하며 snapshot과 대화에 넣지 않는다. 시험·다른 호스트는 선택적 fetch/openSocket 브라우저 primitive를 주입할 수 있다.

| 동작 | 계약 |
|---|---|
| connect | handshake 후 bench.open. 동시 호출은 합치고 같은 세대의 재연결에서 기존 작업대를 유지한다. |
| listAgents | 같은 연결의 agent.list 조회. 화면이 지원 에이전트를 선택할 수 있게 한다. |
| createSession | run.createSession 호출. goal은 label metadata다. ready는 sessionCreated 이벤트 이후다. |
| send | ready 세션에서 사용자 메시지와 전송 결과를 함께 보관한다. 대기·불확실한 상태에서 중복 전송을 거절한다. |
| respondPermission | 선택한 세션의 현재 요청·실제 optionId를 확인한 뒤 같은 bench/run으로 응답한다. 모르는 선택지를 추정하지 않는다. |
| closeSession | 생성 admission 이후 명시적 run.cancel. 동시 종료는 합친다. 새 세대로 사라진 세션은 로컬에서만 닫는다. |
| subscribe / getSnapshot | 불변 snapshot으로 화면을 갱신한다. 구독 해제는 화면 관찰만 멈추고 세션/대화를 유지한다. |
| dispose | 소켓, 갱신/재연결, 화면 구독을 해제한다. run.cancel이나 bench.close는 보내지 않는다. 대기 중 호출은 disposed 오류로 종료한다. |

객체는 화면 pane보다 오래 유지한다. 패널이 접힐 때 unsubscribe하고 다시 펼칠 때 같은 객체에 subscribe하면 대화와 진행 중 응답이 유지된다. 창 종료의 토큰/임대 해제는 공유 Rust desktop 연결 Module이 맡는다.

## 전달 결과와 복구

생성·전송·권한·종료 결과는 기존 `CallOutcome`의 ok, fault(서버의 outcome 포함), notApplied, unknown을 보존한다. 사용자 메시지별 pending/결과를 대화에 남기므로 응답 유실을 전송 성공으로 바꾸지 않는다. 같은 세대에서 같은 멱등 키로 한 번 재시도하는 것은 기존 workbench-client의 계약이다. facade는 추가 prompt 호출을 하지 않으며 새 세대에서는 이전 prompt를 다시 보내지 않는다.

세션 생성이 불확실하면 소유권 검사된 run.replay와 run stream을 읽는다. sessionCreated가 확인되면 사용 가능한 세션으로 표시하되 원래 HTTP 생성 결과 unknown은 유지한다. 불확실한 생성의 종료는 서버의 run 증거를 기다린 뒤 요청하며 10초 내 증거가 없으면 종료 결과도 unknown으로 남긴다. 실행될 수 있는 생성 요청보다 먼저 취소를 보냈다고 완료 처리하지 않는다. 알려진 notApplied 생성은 로컬에서만 닫는다.

replay와 live는 같은 순번 reducer를 사용한다. 중복·이전 순번은 무시하고 상태를 반영한 다음 cursor를 진전시킨다. snapshot 재조회 실패는 streamError로 표시해 입력을 막고 재조회가 성공하면 해제한다. retention gap에서는 이미 표시한 대화와 보관된 이벤트를 이어 받으며 historyIncomplete를 표시한다. 누락한 본문을 만들어 채우지 않는다. 에이전트 error와 stream 복구 오류는 서로 다른 상태로 보관한다.

기존 이벤트 SDK는 cursor 0의 빈 스트림에 gap/event 없이 hello만 오는 재시작도 새 epoch로 알리도록 보강했다. hello의 epoch 관측과 구독 복구 성공은 구분한다. WebSocket이 먼저 재시작을 관측하면 HTTP 연결을 갱신한 뒤 새 작업대를 열도록 한다.

서버 세대가 바뀌면 기존 세션을 lost로 표시하고 권한·구독·bench 대응을 해제한다. 이전 transcript는 남긴다. 새 작업대 연결은 connect에서 명시적으로 수행하며 기존 run id와 prompt는 새 서버로 옮기지 않는다. 종료·재연결 중 도착한 늦은 결과가 다른 세션을 활성화하지 않도록 세대와 run stream/replay 신원을 확인한다.

## AW와 box.md의 조립 위치

AW의 기존 worktree/orchestration 화면은 이미 공유 workbench-client로 HTTP 명령과 WS 이벤트를 사용한다. 해당 화면은 목표 실행·Ralph 반복·child task·panel 소유를 함께 관리한다. 새 agent facade는 단순 채팅의 상태를 공유하는 추가 Module이며 box.md가 이를 소비한다. 두 앱은 같은 HTTP transport와 Rust desktop 연결 수명·ACP server runtime을 공유한다. AW shell의 Rust 연결 Adapter는 gh-21에서 공유 Interface로 전환했다. AW의 고급 orchestration 상태를 단순 채팅 상태로 교체하는 변경은 수행하지 않는다.

## 검증

- 공개 Interface 시험 16개: 동시 connect/close, idle 생성·agent 조회, 두 턴 reducer, 화면 구독 해제·dispose, 동일 키의 단일 prompt 효과, Unknown/NotApplied 보존, 불확실한 생성의 읽기 복구와 종료 상한, 세대 변경, 세션별 권한 선택, 중복 프레임, retention gap, snapshot 실패 후 복구.
- 실제 HTTP/WS 시험 1개: production ACP 엔진과 가짜 ACP 프로세스에서 초기화/session.new만 수행한 idle 상태를 관측했다. 권한 응답을 거친 세 턴 채팅, 소켓 재연결, 숨긴 화면의 대화 유지, dispose 뒤 프로세스 유지, 서버 cancel 뒤 실제 프로세스 종료를 확인했다. `HOST_USE_ACP_ENGINE=1` 시험 옵션을 사용하며 기존 시험 host 기본값은 ScriptedRunEngine이다.
- workbench-client 기존 회귀: unit/type 75개, 실제 HTTP/WS integration 7개 통과.
- 두 공유 패키지 타입 검사, frozen lockfile 설치, Rust 포맷 및 diff 공백 검사를 수행한다. 새 lockfile 변경은 패키지 importer만 추가한다.

box.md UI/네이티브 Adapter 적용과 실제 설치 에이전트 handshake, macOS 화면 검증은 gh-24·gh-25에서 진행한다.
