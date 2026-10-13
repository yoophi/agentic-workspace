# 초기 프롬프트 없는 ACP 세션 생성 HTTP 계약

box.md gh-3의 서브 task gh-22에서 `run.createSession`을 추가했다. 기존 `run.start`의 초기 goal 전송은 유지하며 새 operation만 명시적인 `SessionLaunchIntent::Idle`을 선택한다.

## 호출과 준비 완료

`POST /v1/calls`에서 `operation: "run.createSession"`을 사용한다. 입력은 기존 `RunStartInput`과 같은 `benchId`, `request`, 선택적 `panelId`다. request에는 `agentId`, `cwd`, `agentCommand`, 모델·권한 옵션, 선택적 `runId`와 `mcpServers`를 전달할 수 있다. `goal`은 run의 표시·저장용 metadata이며 이 operation에서는 ACP prompt로 전송하지 않는다. 빈 goal을 전송해 idle을 흉내내지 않는다.

성공 응답의 `AgentRunDto.id`는 예약된 run id다. 이 응답은 비동기 초기화 완료를 의미하지 않는다. 화면은 run stream을 구독하고 `sessionCreated` lifecycle 이후 입력을 허용한다. idle 경로는 ACP initialize·session/new 또는 resume·권한/모델 설정·세션 저장·registry attach가 모두 끝난 뒤 이 이벤트를 발행한다. 따라서 ready 이벤트를 받은 직후 `run.sendPrompt`를 호출할 수 있다.

첫 사용자 메시지는 `run.sendPrompt`로 전달한다. 권한 요청에는 같은 bench/run의 `run.respondPermission`을 사용한다. 활성화된 Ralph loop는 `run.createSession`에서 `invalidArgument`로 거절한다. 초기 프롬프트 없이 자동 반복할 의미가 없기 때문이다.

## 소유권, 복구와 종료

새 operation도 기존 start handler의 작업대 admission, intent-first 원장, run id 예약·중복 검사, launch decorator/MCP 설정, 소유 claim을 사용한다. 같은 멱등 키로 재시도하면 같은 run 결과를 회수하며 세션을 두 번 생성하지 않는다. `run.start`와 구별되는 operation 이름이 원장에 기록된다.

서버 drain 분류는 N(새 작업)이며 비우기 중에는 `draining/notApplied`로 거절된다. 실행 엔진의 작업 관문과 generic 시작 장벽 경로를 유지한다. 초기화 중에는 turn guard를 잡고 ready 이후에는 해제한다. idle 세션이 쉬는 동안 바쁜 turn으로 세지 않는다.

초기화는 idle 경로에서 30초 상한을 두고, 실패·시간 초과를 run error 이벤트로 표시한다. ready는 발행하지 않는다. 초기화 task 취소·시간 초과 시 kill-on-drop로 ACP 프로세스를 종료하고 registry 예약을 정리한다. 준비 중에도 `run.cancel`을 사용할 수 있다. ready 이후 read task가 EOF/오류로 끝나면 오류를 발행하고 프로세스를 종료·회수한 뒤 completed lifecycle을 발행한다. `run.start`의 기존 driver 동작은 유지한다.

## 계약 생성

`pnpm --filter @yoophi/workbench-client generate`가 OpenAPI, 생성 TypeScript 요청·응답 union, `OPERATION_KINDS`를 함께 갱신한다. 새 Rust `export_operation_kinds`는 protocol의 `OPERATIONS`를 정본으로 사용하며 기존 `isQueryOperation` Interface도 유지한다. 계약 hash는 서버가 생성 OpenAPI 내용에서 계산하므로 새 operation을 포함한다. Rust CLI의 닫힌 admission 정책은 유지되며 새 실행 operation도 prerequisite 없이 허용하지 않는다.

## 검증 근거

- `idle_session_http`의 기능 시험 4개: 실제 loopback HTTP/WS + production ACP engine + 가짜 Python agent. 사용자 전송 전 initialize/session/new만 수행, 같은 키 재시도의 단일 launch, ready 직후 두 턴 스트리밍, 권한 응답, 다른 주체 거절, 세션 생성 중 취소·프로세스 종료, 초기화 오류와 기존 run.start 초기 goal, stdout EOF 뒤 프로세스 회수·terminal을 검증했다. 공통 Git fixture 시험을 포함한 출력은 5 passed다.
- ACP core 98개: 기존 launcher·registry·권한·prompt 회귀와 idle 시간 초과 시 실제 agent 종료·ready 미발행을 확인했다.
- protocol 45개, core unit 262개, 공유 desktop 연결 23개를 통과했다.
- core의 drain 분류/입구, bench/run 흐름, epoch 멱등성, 예약 수명 관련 회귀 검사를 통과했다.
- 공유 HTTP/WS TypeScript 클라이언트 74개 및 타입 검사 통과. AW, standalone server, Rust client의 all-targets 컴파일을 확인했다.

실제 설치 에이전트 및 box.md 화면의 HTTP 채팅은 후속 gh-23~gh-25에서 검증한다.
