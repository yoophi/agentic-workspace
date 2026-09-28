# Research: 044 독립 Workbench 서버

기준 main `cb0bd4c`. 현재 조립은 `apps/agentic-workbench/src-tauri/src/lib.rs`가 Tauri 프로세스 안에서 한다. 런타임(`WorkbenchRuntime::bootstrap_with`), HTTP 어댑터(`WorkbenchHttpState::start`), MCP 서버(`McpServerState::start(AppHandle)`), 데스크톱 브리지(`TauriDesktopBridge`: 네이티브 삽입 + MCP launch decorator)가 모두 그 안에 있다. 결합 지점 지도는 이 문서 끝의 부록에 있다.

## R1. 조립 위치: 공유 host crate + 서버 앱

- **Decision**:
  - 새 crate `crates/workbench-host`가 데스크톱과 무관한 조립을 맡는다: 런타임 + HTTP 상태(토큰 발급기·표·resolver) + MCP 서버 + launch decorator + 생명주기(잠금·안내 파일·상태 기계·임대·유휴).
  - 새 앱 `apps/agentic-workbench-server`(Rust bin `agentic-workbench-server`)는 명령줄 해석과 `main`만 갖는다.
  - AW embedded 모드(R11)도 같은 host crate를 쓴다.
- **Rationale**:
  - 헌법 I에 따라 두 소비자(서버 앱, AW embedded)가 쓰는 조립은 crate여야 한다.
  - `workbench-server`는 protocol만 의존한다(server ADR 0001). 조립을 거기 넣으면 그 경계가 깨진다.
- **Alternatives**:
  - `workbench-server`에 bin 추가 → core 의존이 생겨 기각.
  - 서버 앱 안에 조립을 두고 AW가 embedded 모드를 버림 → R11에서 개발 모드를 유지하기로 해 기각.

## R2. MCP 서버와 launch decorator를 창에서 떼기

- **Decision**:
  - MCP 서버 모듈(`src-tauri/src/infrastructure/mcp/*`)을 host crate로 옮긴다. 그 모듈이 `AppHandle`을 쓰는 곳은 런타임을 꺼내는 한 곳뿐이다(`mcp/mod.rs:177-181`). 옮기면서 런타임을 직접 받게 한다.
  - launch decorator는 host crate의 `McpLaunchDecorator`가 된다. 작업대에 창이 있는지 묻지 않고, 모든 run.start에 MCP 연결(`AW_MCP_URL`·`AW_MCP_TOKEN`·`AW_MCP_RUN_ID`, 헤더, 안내문)을 넣는다.
- **Rationale**:
  - 오늘 decorator가 `label_for(bench)`로 창을 요구하는 까닭은 decorator가 Tauri 브리지에 붙어 있어서다. 서버에는 창이 없다.
  - MCP 토큰은 run에 묶이고, 작업대 닫기가 폐기한다(`bench_service.rs:260-264`). 창과 무관하다.
- **Alternatives**: 창이 있는 작업대에만 MCP 연결 → 데스크톱 없이 시작한 run(소유자 클라이언트)이 MCP를 못 써 기각.

## R3. 네이티브 삽입 전달 제거, 창 제목은 데스크톱이 적용

- **Decision**:
  - 외부 서버 모드의 host는 `DesktopBridge::deliver`를 하지 않는다. no-op 브리지를 쓴다.
  - 창 제목 요청은 이미 `bench:<id>` 스트림의 `bench.titleRequested.v1`로 발행된다(040). 화면은 043부터 이를 구독한다(`network-events.ts` TITLE_EVENT).
  - 화면이 제목 이벤트를 받으면 새 데스크톱 command `apply_window_title(title)`을 불러, 창 제목과 네이티브 메뉴 동기화를 한다(오늘 브리지가 하던 일, `tauri_desktop_bridge.rs:147-177`).
- **Rationale**: 서버는 창을 모른다. 표현 적용은 데스크톱 몫이다(FR-004·019).

## R4. 단일 writer: 잠금·안내 파일

- **Decision**:
  - 데이터 디렉터리 아래 `workbench/server/`에 둔다(디렉터리 0700):
    - `owner.lock`: 서버가 실행 내내 배타 잠금.
    - `startup.lock`: 시작 절차의 배타 잠금.
    - `server.json`: 안내 파일(0600).
  - 잠금은 표준 라이브러리 파일 잠금(`File::try_lock`/`lock`, Rust 1.89+)을 쓴다. 프로세스가 죽으면 OS가 푼다.
  - 안내 파일은 같은 디렉터리의 임시 파일(0600으로 생성)에 쓰고 `fsync` → `rename`으로 원자적으로 바꾼다. 서버는 **준비된 뒤에만** 안내 파일을 쓰고, 정지 때 자기 인스턴스 것일 때만 지운다.
  - "OS 사용자 + 채널 + 프로필 + 데이터 디렉터리" 구분은 데이터 디렉터리 경로로 이뤄진다. 앱 identifier별 app-data 디렉터리는 사용자별·채널별로 다르다(예: 스모크의 `…smoke043`). 경로는 정규화(canonicalize)해 쓴다.
- **Rationale**:
  - 잠금이 데이터 디렉터리 안에 있으면 다른 프로필·채널이 서로를 가로챌 수 없다(설계 문서 "discovery와 single writer").
  - 파일 잠금은 비정상 종료 뒤에도 남지 않는다. 남는 것은 안내 파일뿐이고, R5가 판별한다.
- **Alternatives**:
  - PID 파일만 → PID 재사용 문제로 기각.
  - 소켓 바인드로 단일성 → 임의 포트 정책과 충돌해 기각.

## R5. 시작 절차(ensure)·복구·버전 확인

- **Decision**:
  - **자격 증명을 보내기 전에 서버 신원부터 확인한다(설계 리뷰 D1)**: 안내 파일이 남아 있고 원래 서버가 죽었다면, 그 포트를 다른 프로세스가 차지하고 있을 수 있다. 확인 없이 소유자 자격 증명을 보내면 그 프로세스에 자격 증명이 새어 나간다.
    - 인증 없는 `POST /v1/system/identify {nonce}` → `{instanceId, proof}`. `proof`는 `hex(HMAC-SHA256(key = ownerToken 문자열의 UTF-8 바이트, msg = nonce + "\n" + instanceId))`다(고정 벡터는 contracts/server-lifecycle.md §3).
    - 클라이언트는 안내 파일의 `ownerToken`으로 `proof`를 검증한다. 맞을 때만 bearer로 소유자 자격 증명을 보낸다.
    - 서버는 `ownerToken`을 알기 때문에 증명을 만들 수 있다. 자격 증명 자체는 오가지 않는다.
  - 클라이언트 `ensure(data_dir, exe)`:
    1. `startup.lock` 획득(상한 대기, 기본 20초).
    2. `server.json`이 있으면 `identify`로 신원을 증명받은 뒤에만 버전 확인과 **소유자 인증 상태 조회**를 한다. 인스턴스 식별자가 일치하고 준비 상태이면 붙는다.
    3. 확인이 실패하면 `owner.lock`을 잠깐 `try_lock`해 본다. 잡히면 서버가 없다는 뜻이다. 남은 안내 파일을 지우고 풀어 준 뒤 서버를 띄운다. 잡히지 않으면 서버는 살아 있지만 준비 전이거나 비우는 중이다. 준비나 정지를 기다린다.
    4. 서버를 띄운 뒤 안내 파일이 생기고 확인을 통과할 때까지 기다린다.
    5. `startup.lock` 해제.
  - 서버 시작:
    1. `owner.lock` `try_lock`. 실패하면 기존 안내 파일 내용을 알리고 종료 코드 3.
    2. 데이터 디렉터리를 열고 시작 복구(ledger reconciler)를 한다.
    3. 끝점을 연다.
    4. 준비 → 안내 파일을 쓴다.
  - 서버는 데스크톱 프로세스 그룹과 분리해 띄운다(`CommandExt::process_group(0)`, 표준 입출력은 null, 로그는 데이터 디렉터리의 파일). 앱이 끝나거나 `tauri dev`가 Ctrl+C로 그룹에 신호를 보내도 서버는 산다.
- **Rationale**:
  - PID는 쓰지 않는다. 살아 있는지는 잠금 소유로, 어느 인스턴스인지는 인증된 확인으로 판단한다(FR-008·009).
  - 서버 둘이 동시에 떠도 `owner.lock`이 하나만 데이터를 열게 한다. `startup.lock`은 불필요한 기동을 줄이는 최적화이지, 정확성 조건이 아니다.
- **Alternatives**:
  - OS 사용자 서비스(launchd) → 설치·업데이트 소유자 설계(f)가 필요해 다음 증분으로.
  - 이중 fork daemonize → `process_group`과 null stdio로 충분해 기각.

## R6. 소유자 주체와 창 토큰 발급

- **Decision**:
  - 안내 파일에 32바이트 무작위 **소유자 자격 증명**을 소문자 hex(64자)로 넣는다(파일 0600).
  - 이 자격 증명의 주체는 새 `PrincipalKind::Owner`(`local:owner`)다. 가진 권한은 모든 scope와 `server:admin`, 그리고 **작업대 소유 판정 우회**(모든 작업대 조회·구독·닫기·run 취소)다.
  - 창 토큰은 소유자가 owner 전용 operation `desktop.issueWindowToken`으로 받는다. 입력은 `{label, incarnation, origin}`이고, 출력 `{token, expiresAt}`은 043과 같은 창 주체·출처 묶음이다.
  - 창 폐기는 `desktop.retireWindow {label, incarnation, closeBench}`다. 한 호출로 그 주체의 토큰·표 폐기와 (요청 시) 그 주체가 연 작업대 닫기를 한다.
  - 서버의 창 주체 등록은 두지 않는다. incarnation은 데스크톱이 발급하고, 서버는 소유자가 요청한 주체로 토큰을 만든다.
  - **폐기 tombstone(Codex 설계 리뷰 C4)**: 발급 요청과 `retireWindow`가 HTTP로 따로 오므로, 폐기보다 늦게 도착한 발급 요청이 폐기된 창의 토큰을 되살릴 수 있다. 그래서 서버는 세대 동안 폐기한 창 주체(`label:incarnation`)의 tombstone을 둔다.
  - **폐기된 주체의 늦은 호출(Codex 구현 리뷰)**: 토큰 tombstone만으로는 폐기 **전에** 헤더를 인증한 요청(본문 대기)이 폐기 뒤 런타임에 도착하는 것을 막지 못한다.
    - 그래서 `retireWindow`는 닫기 전에 작업대 레지스트리에 그 주체를 폐기로 표시한다.
    - 런타임 입구가 폐기된 주체를 `unauthenticated`로 거절한다.
    - 작업대 등록은 표시와 같은 잠금 아래에서 검사·삽입해, 입구를 먼저 지난 호출도 새 작업대를 만들지 못한다.
    - 발급과 폐기는 발급기의 같은 잠금 아래에서 처리한다.
    - tombstone이 선 주체에는 발급하지 않는다(`forbidden`, "window is retired").
    - 같은 label의 새 incarnation은 다른 주체라 영향을 받지 않는다.
    - 시험: 폐기가 먼저 끝난 뒤 지연된 발급이 도착하는 순서, 그리고 같은 label의 새 incarnation 발급.
- **Rationale**:
  - 소유자 자격 증명은 같은 OS 사용자만 읽을 수 있다. 데스크톱은 그 사용자의 프로세스라 같은 신뢰 수준이다.
  - 데스크톱이 없을 때 run을 조회·취소할 주체가 필요하다(US1, 완료 기준 (a)).
  - 창별 격리는 043처럼 토큰 단위로 유지한다(창 토큰은 소유자 권한을 갖지 않는다).
- **Alternatives**:
  - 서버가 창 incarnation을 등록 → 데스크톱이 이미 창 수명을 안다. 서버 등록은 중복이고 늦은 정리 경합만 늘려 기각.
  - 소유자 = 기존 `desktop` 주체 재사용 → 작업대 소유 우회를 쓰는 주체를 명시적으로 드러내려고 새 kind로.

## R7. 종료 상태 기계와 비우기(drain) 분류 — 새 작업 대 끝내는 제어 (사용자 검토 1·3)

- **Decision (상태)**: `serving` → `draining{mode: wait|idle}` → `stopping` → 종료. `stopping`에서는 042처럼 새 호출을 모두 `503`으로 거절하고, 받아들인 호출을 drain한다.
- **Decision (입구 분류)**: `draining`에서 Workbench 호출 입구는 operation마다 네 가지로 처리한다. 전체 표는 `contracts/drain-classification.md`.
  - **Q(조회)**: 받는다. operation 종류가 query면 자동으로 Q다(protocol `spec_for(op).kind`).
  - **C(끝내는·해제하는 제어)**: 받는다. 활성 작업을 줄이거나 이미 있는 작업을 마무리만 한다.
  - **K(이어 가기, 조건부)**: 입력이 **이미 있는 대기 항목**을 가리키고, 서버가 그 항목이 살아 있음을 확인할 때만 받는다. 아니면 N과 같다.
  - **N(새 작업)**: `draining` fault로 거절, 적용 안 됨.
  - 분류는 core의 `drain_class(OperationId, &input)`가 command마다 **빠짐없는 match**로 정한다. 새 operation을 추가하면 분류하지 않고는 컴파일되지 않는다.
  - 입구는 `WorkbenchRuntime::call` 하나다. HTTP·MCP·embedded가 같은 판정을 받는다.
- **K 경로 1 — 교환 전달(사용자 검토 3, Codex 설계 리뷰 C2·C5)**:
  - 교환은 서버가 agent에 직접 보내지 않는다. 043의 실제 순서는 다음과 같다:
    1. 원장이 요청 이벤트를 받아 대상 패널로 **라우팅**한다(React 상태 갱신, `worktree-agent-run-area.tsx:315-337`).
    2. **곧바로 확인(ack)**을 보낸다(`exchange-reconciler.ts:45-55`).
    3. 패널은 prompt를 화면 대기열에 넣는다(`agent-run-panel.tsx:1458-1471`).
    4. 대상 run의 현재 turn이 끝난 뒤 별도 effect가 `run.sendPrompt`로 보낸다(`:1042-1064`, 키 `exchange-delivery:<requestId>`).
    - 따라서 확인이 실제 전송보다 **먼저** 도착한다. 교환 상태(`accepted`/`delivered`)로는 "아직 prompt가 안 갔다"를 알 수 없다.
  - 계약:
    - 서버는 교환마다 **전달 prompt 소비 여부**(`deliveryConsumed`)를 따로 관리한다.
    - **소비는 엔진 대기열 수락과 묶는다(Codex 재검토 E2)**: `SendPromptUseCase`는 전송을 spawn하고 곧바로 성공을 돌려준다(`send_prompt.rs:46-58`). 그 사이 다른 prompt(예: coordinator 알림)가 먼저 turn을 잡으면 전송이 실패하고 Error 이벤트만 남는다(`runner.rs:787-790`). 소비를 이 성공에 묶으면 교환을 잃는다. 그래서 `continuation`이 붙은 `run.sendPrompt`는 **엔진 대기열 경로**(`queue_prompt`: 현재 turn 뒤 차례로 보냄, 바쁨으로 실패하지 않음)로 보낸다. 소비 표시와 대기열 등록을 한 번에 한다. 그 prompt의 활동 예약(아래 실행 수명 계약)이 전달이 끝날 때까지 활성 작업에 남는다. 대기열 전달이 run 종료로 실패하면 소비 표시는 남긴다(대상 run이 없어 교환의 뜻이 사라졌다). 이 경우 `server.status`의 `failedExchangeDeliveries`로 보고한다.
    - `run.sendPrompt` 입력에 선택 필드 `continuation: { exchangeRequestId }`를 더한다.
    - `draining` 중에는 다음을 **원자적으로** 모두 확인하고 `deliveryConsumed`를 세울 때만 받는다:
      - 그 교환이 호출자 작업대에 있다.
      - 대상 run이 이 run이다.
      - 교환 배달 방식이 `send`/`queue`다(`draft`는 사용자가 직접 보내야 하므로 K가 아니다).
      - 확인 결과가 `rejected`가 아니다.
      - `deliveryConsumed`가 아직 없다.
      - 멱등성 키가 정확히 `exchange-delivery:<requestId>`다.
    - 같은 교환으로 두 번째 prompt(다른 키·다른 내용·동시 요청 포함)를 보내면 N으로 거절한다. 같은 키 재시도는 기존 멱등 결과를 돌려준다.
    - prompt 내용은 교환 메시지에 묶지 않는다. 비우기 분류는 보안 경계가 아니다(그 창은 비우기가 아닐 때 어떤 prompt든 보낼 수 있다). 유한성에는 소비 1회로 충분하다.
    - `serving`에서도 `continuation`이 있으면 `deliveryConsumed`를 세운다. 그래야 비우기 시작 뒤에 같은 교환으로 또 보내지 못한다.
  - 화면의 교환 전달은 이 필드를 싣는다(오늘 키와 함께).
- **K 경로 2 — 대기 task 배정(Codex 설계 리뷰 C1)**:
  - 동시 실행 상한으로 대기한 자식 task는 서버가 자동 실행하지 **않는다**. 스케줄러 `release`는 다음 준비 task id를 돌려줄 뿐이다(`agent_tools.rs:555-570`). coordinator가 `orchestration.assignChildTask`를 다시 불러야 진행한다(`orchestration_liveness.rs:232-266`).
  - 계약: `draining` 중 `orchestration.assignChildTask`는 대상 task가 **비우기 시작 전에 만들어진 대기 task**일 때만 받는다(K). 새로 만든 task는 `createChildTask`가 N이라 생기지 않는다.
  - **배정의 원자성(Codex 재검토 E3, 기존 결함 포함)**: 오늘 배정 경로는 원자적이지 않다. `scheduler.acquire`는 이미 active인 task에도 `Acquired`를 돌려준다. 두 요청이 `current_run_id`가 없는 같은 snapshot을 읽으면 둘 다 기동할 수 있고, `reserve_child_run`(`service.rs:802-828`)은 상태·기존 예약을 보지 않고 덮어쓴다. 비우기와 상관없이 같은 task에서 run이 둘 뜨고 한 run의 보고 권한이 밀려날 수 있다.
    - 수정: 배정은 orchestration 저장소의 단일 RMW 경계(core ADR 0006) 안에서 **비교 후 변경**한다. 조건은 task가 대기(`Ready`)이고 `current_run_id`가 없는 것이고, 변경은 `Starting` 예약이다. 비우기 중이면 "비우기 시작 전 생성" 판정도 같은 경계에서 한다.
    - 이미 예약이 있으면 기동하지 않고 기존 예약 결과를 돌려준다. `reserve_child_run`도 기존 예약·상태를 검사한다.
    - 시험: 서로 다른 요청 키의 동시 배정(run 1개만), 배정과 취소의 경합, 비우기 중 K 배정.
  - 시험: 동시 실행 상한 1에서 task 둘을 받아들인 뒤 wait-stop → 첫 task 완료 → coordinator가 둘째를 배정(K) → 완료 → 서버 정지.
- **서버 내부로 이어지는 경로(입구 판정 없음)**:
  - coordinator 알림 전달(`notification_dispatcher`)
  - 대기 자식 명령(`PromptDelivery::Queue`, 엔진 대기열 `engine_agent_worker.rs:197`)
  - 엔진이 현재 turn 뒤 대기열 prompt를 넘기는 것
- **활성 작업(`ActiveWork`)과 출처(설계 리뷰 D5, Codex 설계 리뷰 C3)**:
  - **바쁜 run 수**(`busyRuns`): 진행 중 turn, 엔진 대기열 prompt, 권한 대기 중 하나라도 있는 run.
    - 세션 수(`active_run_count` = `runs.len()`)는 쓰지 않는다. ACP 프로세스는 turn이 끝나도 다음 prompt를 기다리며 살아 있다(`runner.rs:419-475`, `start_agent_run.rs:87-88`). 세션 수로 세면 wait·유휴가 영원히 끝나지 않는다. 그 accessor는 시험 전용이기도 하다.
    - **출처는 이벤트 추정이 아니라 실행 수명 계약이다(Codex 재검토 E1)**. 이벤트로는 정확히 셀 수 없다: `queue_prompt`는 spawn하고 곧바로 돌아가며 등록 이벤트가 없고(`acp_run_engine.rs:125-148`), `PromptSent`는 잠금을 잡은 뒤에야 나온다(`runner.rs:794-799`). RPC 오류는 `PromptCompleted` 없이 돌아간다(`runner.rs:748`). 시작 중인 run과 Ralph 반복 사이의 지연도 이벤트로 드러나지 않는다.
    - 계약(`RunActivity`, core):
      - 모든 prompt 실행 진입점에서 **동기적으로 예약**하고, 그 실행 future가 끝날 때(성공·오류·취소·abort로 drop) guard가 해제한다. 진입점은 엔진의 `start`, `send_prompt`, `queue_prompt`, `steer`, `send_and_wait`, orchestration 작업자와 알림 전달기가 부르는 엔진 경로다.
      - `send_prompt`는 엔진이 `SendPromptUseCase` 대신 세션의 `send_prompt` future를 직접 spawn해 guard를 그 future에 묶는다(오늘 `queue_prompt`와 같은 모양).
      - **시작의 초기 prompt 순서(Ralph 반복 포함)는 acp-agent-core runner 안에서 끝난다.** runner가 순서를 시작할 때 받는 guard를 순서 끝(`child.wait()` 전)에서 놓게 한다. `acp-agent-core`의 `StartAgentRunUseCase`·runner에 선택 인자(활동 guard 공급자)를 더한다. 다른 소비자(ask-code·hushline)는 인자를 넘기지 않아 동작이 같다. 헌법 V에 따라 두 앱의 Rust 검사·시험을 게이트에 넣는다.
      - 권한 대기는 그 prompt 실행 future 안에서 일어나므로 같은 guard에 포함된다.
    - **정지 판정과 예약의 직렬화**: 활동 표는 서버 상태와 한 잠금을 쓴다. 정지 판정은 그 잠금 아래에서 "활동 0이면 `stopping`으로 전이"를 한다. `stopping` 뒤의 예약은 실패한다(내부 경로의 새 실행은 시작하지 않고 run 취소로 처리). 예약이 정지 판정과 엇갈려 "0으로 보고 멈췄는데 방금 예약된 실행이 있는" 경우가 없다.
    - 쉬고 있는 세션(바쁘지 않은 run)은 활성 작업이 아니다. 서버가 멈출 때(wait·idle·force 모두) 남은 세션은 취소되고 run은 끝난다.
  - 진행 중(배정된) orchestration task 수 + 비우기 시작 전에 만든 대기 task 중 **배정할 쪽이 있는 것**의 수.
  - **구현 중 정책 변경(메인 세션 검토, 사용자 검토 요청으로 근거·반례 기록)**: 처음 계약은 "비우기 전 대기 task는 활동 작업"이었다. 그러면 coordinator가 쉬는 동안 대기 task 하나가 wait·유휴 정지를 **영원히** 막는다.
    - 사실(코드): `assignChildTask`는 coordinator 역할의 agent 도구뿐이다(`agent_tools.rs:463` `AgentRole::Coordinator`). 소유자·데스크톱·CLI는 agent 전용 operation을 부를 수 없다(T027 소유자 우회 제외). 따라서 배정은 coordinator turn 안에서만 일어난다.
    - coordinator turn이 생기는 경로: 사용자 prompt(비우는 중에는 N), 저장된 미전달 coordinator 알림(알림 전달기), 엔진 대기열 prompt·Ralph 반복(이미 바쁜 run).
    - 결정: 대기 task는 **coordinator run이 살아 있고, 바쁘거나 그 coordinator에게 미전달 알림이 있을 때만** 활동으로 센다. 그 밖의 준비 task는 `server.status`의 `deferredTasks`(task id)로 보고만 한다. task는 orchestration 저장소에 남아 작업대가 닫히면 복구할 수 있는 작업 영역이 된다(041). 따라서 정지가 작업을 잃지 않는다.
    - 데스크톱 임대는 조건이 아니다. 비우는 중 데스크톱은 N인 prompt를 못 보내 배정을 일으킬 수 없다(교환과 다른 점 — 교환은 데스크톱 원장이 직접 전달한다). 임대가 있는 서빙 상태는 유휴 판정 자체가 일어나지 않는다.
    - 데스크톱 operation도 배정을 일으키지 않는다: `orchestration.retryTask`·`reassignTask`·`recover`·`dispatchPrompt`는 N이다(`drain.rs`). 비우는 중 데스크톱은 준비 task를 시작할 수 없다.
    - 실제 K 경로: 자식 결과 → 알림 전달기가 coordinator turn을 연다(`send_and_wait`, A-turn) → 그 turn 안에서 agent가 `aw_assign_child_task`(K)를 부른다. turn이 끝날 때까지 알림은 전달 중이라 미전달로 세지고, turn 중 배정은 비우는 중에도 받아들여진다.
    - 반례·검증:
      - core(`crates/workbench-core/tests/server_stop.rs`): 배정할 쪽 없음 → `deferredTasks`·유휴 정지 / 임대만 있음 → wait 정지 완료 / coordinator 바쁨 → 셈 / 전달기를 붙잡아 미전달 알림 → 셈. 변이 3개(항상 셈, 임대면 셈, 바쁨 조건 제거)가 각각 해당 시험을 실패시킨다.
      - host 실제 경로(`crates/workbench-host/tests/wait_stop.rs`, HTTP·MCP·감시 루프): (d) 알림이 연 coordinator turn 안에서 배정 → turn 중 `queued_tasks=1`·멈추지 않음 → 결과 → 멈춤. (d 대조) coordinator turn이 배정 없이 끝남 → 멈추기 직전 파생 `deferred_tasks=[task]`·`queued_tasks=0` → 멈춤 → **host 전체 종료 → 같은 데이터 디렉터리로 새 host** → `listRecoverable`에 같은 task id·`ready`·`startedAt=null` → `bootstrap{resumeWorkspaceId}` → 새 Main run `handoffCoordinator` → `recover` → 새 coordinator가 배정 → 새 run → 결과 → `completed`.
      - 한계: 재시작은 같은 테스트 프로세스 안의 host 재조립이다(OS 프로세스 재시작 아님). 독립 서버 바이너리에는 가짜 엔진이 없어 MCP를 부르는 agent를 쓸 수 없다 — 프로세스 수준 재배정은 미검증으로 남긴다.
    - 이 검증에서 드러난 기존 결함(041부터, 044 변경 아님): `recover`의 `scheduler.reconcile`이 준비 task를 자리와 무관하게 대기열에 넣고, `acquire`는 대기열에 있는 task에 자리가 비어도 `Queued`를 돌려줬다. 실행 중 task가 없으면 `release`가 오지 않아 재시작 뒤 재배정이 영원히 대기했다. 수정: 대기열의 task도 자리가 비면 `acquire`가 시작한다(`scheduler.rs`, 단위 시험 red→green, 변이 시 host 재시작 시험 실패).
    - 처음 제안의 잘못: 메인 세션이 처음 둔 조건 "데스크톱 임대 또는 coordinator 바쁨"은 틀렸다(임대는 배정 주체가 아님). 사용자 반론 뒤 코드로 확인해 고쳤다.
    - 구현 리뷰 대상으로 명시한다(정책 변경·scheduler 수정·증거 범위).
  - **OCR 구현 리뷰 반영(정지 계약 변경, 구현 리뷰 대상)**:
    - 비우기가 시작된 뒤 만든 준비 task(비우기 전에 받은 호출이 비우기 안에서 만든 것)는 이어 가기로 배정받지 못한다(`ensure_assign_continues`). 활동으로 세지 않고 `deferredTasks`로 보고한다. 이전에는 보고에서 빠졌다. 입구 판정과 C-call 예약도 관문의 한 잠금 아래에서 한다(`WorkGate::admit`).
    - **알림 재시도 상한**: coordinator turn이 계속 실패하면(인증·할당량 등) 재시도 가능한 실패 알림이 영원히 활동이라 wait·유휴 정지가 끝나지 않았다(재시도는 50ms부터 두 배, 최대 5초 간격, 횟수 제한 없음). 결정:
      - 재시도를 **기다리는** 재시도 가능 실패 알림은 `attemptCount < MAX_NOTIFICATION_ATTEMPTS_FOR_STOP`(3)일 때만 활동으로 센다. 넘으면 `stalledNotifications`로 보고만 한다.
      - `pending`·`dispatching`(진행 중인 시도, N-notify 예약)은 시도 수와 무관하게 활동이다. 상한은 진행 중 시도에 적용하지 않는다.
      - 시도 수는 "전달됨"이 아니다. 알림은 `failed`·재시도 가능 그대로 저장되고 거두지도 지우지도 않는다. 배경 재시도는 서빙 중 계속된다.
    - 증명 범위(`crates/workbench-core/tests/server_stop.rs`):
      - 주입한 전달 실패 → 재시도 → 상한 전에는 정지를 막는다 → 상한 뒤 유휴 정지가 진행되고 id가 `stalledNotifications`에 든다. 저장은 `failed`·재시도 가능, 거두지 않음.
      - 상한을 넘긴 뒤의 진행 중 시도(`dispatching`)는 정지를 막는다.
      - 같은 runtime에서 coordinator가 다시 성공하면 배경 재시도가 전달해 `delivered`가 된다.
      - 정지 뒤 같은 데이터로 runtime을 재조립하면(같은 시험 프로세스, OS 프로세스 재시작 아님) 알림은 `failed`·재시도 가능으로, 그 보고는 결과로 저장돼 있다. 복구 → 새 coordinator 인계 뒤 알림은 `superseded`가 되고, 결과는 `orchestration.collectReports`로 읽힌다.
    - 한계(증명하지 않은 것):
      - 재시작 뒤 그 알림이 새 coordinator에게 **자동으로 다시 전달되지는 않는다**. 인계는 이전 세대 알림을 `superseded`로 바꾸고(041 계약), 새 coordinator의 `collectChildResults`는 이전 세대의 끝난 task를 받지 않는다(활성 세대 한정).
      - 상한 뒤 N+1번째 시도에서 성공할 coordinator라도 정지가 먼저 오면, 그 알림은 전달되지 않은 채 저장된다. 재시작 뒤에는 인계로 `superseded`가 된다. 알림이 가리키는 보고는 저장돼 보고 모으기로 읽히지만, coordinator가 그 알림을 받지는 못한다.
  - **확인했지만 전달 prompt가 아직 소비되지 않은 교환**(`send`/`queue`, `rejected` 아님, 대상 run 살아 있음): **데스크톱 임대가 하나라도 있을 때만** 센다. 임대가 없으면 화면 대기열을 보낼 클라이언트가 없어 기다려도 끝나지 않는다. 이 경우 `server.status`의 `undeliverableExchanges`로 보고한다.
  - 이 프로세스가 적용 중인 ledger `pending` 수
  - 받아들인 분리 호출 수(HTTP·MCP)
  - 유효 임대 수(유휴 판정에만)
- **wait-stop이 끝남을 보장하는 논증**:
  - 새 run turn을 시작하는 호출은 K를 빼면 모두 N이다: run.start, sendPrompt(조건 밖), steer, cancelAndSend, orchestration.dispatchPrompt·sendChildCommand·sendChildMessage·delegateGoal·createChildTask·assignChildTask(조건 밖)·retry·reassign·recover.
  - K는 비우기 시작 때 이미 있던 항목에만, 항목마다 **한 번** 적용된다: 교환마다 전달 prompt 1회(`deliveryConsumed`), 대기 task마다 배정 1회. 새 교환(`exchange.send`·`sendFromRun`)과 새 task(`createChildTask`)는 N이라 늘지 않는다. 따라서 K로 생기는 turn 수는 유한하다.
  - 알림 전달기가 만드는 coordinator turn 안에서 새 작업을 만드는 호출은 N이다.
  - 사용자는 언제든 "강제" 정지로 넘어갈 수 있다.
- **Rationale**:
  - 모든 변경을 거절하면 권한 응답·교환 전달과 확인·orchestration 보고가 막혀, 기다리는 작업이 끝나지 않는다(사용자 지적).
  - K는 "새 작업"과 "이미 약속된 작업의 전달"을 입력으로 구분한다.
- **Alternatives**:
  - 조회만 허용 → wait-stop이 끝나지 않을 수 있어 기각.
  - 호출자 kind로 판정(agent 호출은 모두 허용) → agent도 새 작업을 만들 수 있어 기각.
  - 비우기 시작 때 확인 전 교환을 서버가 `rejected: draining`으로 종결 → 이미 받은 메시지를 잃어 기각.
- **검증(추상 C 호출만이 아니라 실제 전달 경로로)**:
  1. 분류 표 문서와 `drain_class`, `OperationId::ALL`, operation 종류의 대조 시험(문서 표 파싱).
  2. operation마다 `draining` 입구 판정. K는 조건 충족·불충족 양쪽을 본다.
  3. **실제 경로 wait-stop 시험**: 실제 서버 조립(시험 host), HTTP 클라이언트, 043의 실제 소비자 코드를 쓴다. 각 경우 wait-stop을 요청한 뒤 실제 경로로 작업을 끝내고, 서버가 멈추는지 본다:
     - (a) 권한 대기 run: 권한 응답 → turn 완료 → **세션은 살아 있어도** 바쁜 run 0 → 멈춤(C3 실패 시나리오: 세션 수로 세면 멈추지 않음)
     - (b) 교환 전달: 대상 run이 바쁜 상태에서 교환 요청 → 043 원장이 라우팅하고 **전송보다 먼저 확인** → turn 종료 뒤 패널 대기열이 `run.sendPrompt(continuation)` → 엔진 대기열 등록·소비 → 전달 완료 → 멈춤(C2 실패 시나리오: 확인 뒤 상태로 판정하면 전달 거절). 변형: 패널 전송 직전에 coordinator 알림 prompt가 먼저 turn을 잡아도 교환 prompt가 그 뒤에 전달된다(E2 실패 시나리오: 즉시 전송 경로면 Error만 남고 유실)
     - (c) orchestration 자식 task 보고·결과(MCP 경로) → 알림 전달기가 coordinator에 전달 → task 종결 → 멈춤
     - (d) 동시 실행 상한 1, task 둘 → 첫 task 완료 → coordinator가 둘째를 `assignChildTask`(K) → 완료 → 멈춤(C1 실패 시나리오: N이면 멈추지 않음)
     - (e) 대기 자식 명령(`queue`): 엔진 대기열 prompt 전달 → 완료 → 멈춤
  4. 각 실제 경로 시험의 대조 변이: 해당 C·K를 N으로 바꾸면(또는 (a)에서 세션 수로 세면) wait-stop이 정해진 상한 안에 끝나지 않음을 단정한다.
  5. **K 단일 실행(C5)**: 같은 교환으로 다른 키·다른 내용의 둘째 prompt, 동시 두 요청, `draft` 교환, `rejected` 교환, 다른 run 대상 → 모두 거절되고 효과는 1회뿐임을 확인한다. 같은 키 재시도는 기존 결과를 돌려준다.
  6. N operation이 `draining`으로 거절되고 효과가 없는 시험.
  7. ledger `unknown`만 남은 서버가 wait·idle에서 멈추고 `unresolvedOperations`에 보이는 시험.
  8. 데스크톱 임대가 없을 때 미소비 교환이 wait를 막지 않고 `undeliverableExchanges`에 보이는 시험.
  10. **실행 수명 계약(E1)**: 대기열 prompt만 남은 순간(이전 `PromptCompleted`와 다음 `PromptSent` 사이) 바쁜 run이 0이 아님. RPC 오류로 끝난 prompt 뒤 바쁜 run이 0. 시작 중·Ralph 반복 사이에 0이 아님. 정지 판정과 동시에 예약해도 "멈춘 뒤 실행"이 없음.
  11. **배정 원자성(E3)**: 서로 다른 키의 동시 배정 100회에서 run은 task마다 1개, 배정과 취소의 경합.
  9. **창 폐기 단조성(C4)**: 폐기 완료 뒤 지연된 발급 → `forbidden`. 같은 label 새 incarnation 발급 → 성공. 발급·폐기 동시 100회 → 폐기 뒤 유효 토큰 0.

## R14. 작업 예약의 원자성 경계와 상태 전이 표 (사용자 검토 5, Codex 재검토 E1–E4)

R7에 흩어져 있던 조건(활동 예약, 교환 전달 수락, task 기동 예약, 정지 판정)을 **한 경계**로 묶는다. 개별 조건을 덧붙이는 대신, 모든 전이를 아래 표 하나로 정한다.

### 경계: 작업 관문(`WorkGate`, core)

- 한 잠금 G 아래에 다음을 둔다: 서버 상태(`serving`·`draining{mode}`·`stopping`), 활동 예약 표(예약 id → 종류·run), 교환 전달 소비 표, task 기동 토큰 표.
- G는 짧게만 잡는다(await를 걸치지 않음). 비동기 작업(엔진 등록·대기열 등록·저장소 쓰기)은 **G 아래에서 예약을 먼저 만들고**, G 밖에서 수행하고, 결과를 다시 G 아래에서 확정·해제한다.
- 예약은 **드롭하면 해제되는 guard**다. 성공·오류·취소·abort(future drop) 어느 쪽으로 끝나도 해제가 빠지지 않는다.
- 정지 판정은 G 아래에서만 한다. 판정 뒤(`stopping`)의 예약은 실패한다.

### 예약 종류

| 종류 | 만드는 곳 | 정확히 무엇을 덮나 |
|---|---|---|
| A-turn | 엔진의 prompt 실행 진입점(`start` 초기 순서·`send_prompt`·`queue_prompt`·`steer`·`send_and_wait`, orchestration 작업자·알림 전달기 경로) | 그 prompt 실행 future 전체(권한 대기 포함). 초기 순서는 runner가 순서 끝에서 놓는다 |
| X-deliver | `run.sendPrompt(continuation)` | 교환 전달 prompt의 엔진 대기열 등록부터 그 prompt 실행이 끝날 때까지(등록 뒤 A-turn으로 인계) |
| T-start | `orchestration.assignChildTask`(대기 task 배정) | `Starting` 예약부터 엔진 실행 허용까지(허용과 함께 A-turn으로 인계) |
| N-notify | 자식 보고·결과·막힘·입력 요청이 coordinator 알림을 저장하는 순간(보고 호출의 C-call을 놓기 전) | 알림 선택·전달·**결과 저장 commit까지**. A-turn과 **별개로** 유지한다(인계하지 않음). `send_and_wait`는 그 안에서 A-turn을 따로 잡고 prompt 실행이 끝나면 놓는다. 전달 시도의 소유권(`attemptId`)은 N-notify가 쥔다 |
| C-call | HTTP·MCP 받아들인 분리 호출 | 호출 처리 끝까지(042) |

활동 작업 = 예약 수 합계 + 이 프로세스의 ledger `pending` + (데스크톱 임대가 있을 때) 미소비 교환 + 비우기 시작 전 대기 task 중 배정할 쪽(살아 있는 coordinator가 바쁘거나 미전달 알림이 있음)이 있는 것(R7 구현 중 정책 변경; 나머지는 `deferredTasks`로 보고만) + **저장된 미전달 coordinator 알림 중 대상 coordinator run이 살아 있는 것**(저장소에서 파생, Codex 재검토 3 F2).

### 엔진 시작 장벽 (Codex 재검토 3 F1)

- 오늘 `StartAgentRunUseCase::execute`는 `reserve_run().await` → `tokio::spawn`(곧바로 launcher 실행) → `attach_run_handle().await` 순서다. 토큰 전이를 등록 뒤에 하면 그 사이 성공한 취소가 이미 spawn한 실행을 막지 못한다. 등록 전에 하면 취소가 아직 registry에 없는 run을 "취소"하고 끝난다(`cancel_run`은 없는 run에 false).
- 그래서 엔진 시작을 **준비**와 **실행 허용**으로 나눈다(`acp-agent-core` 시작 경로에 선택 인자 `start_gate` 추가. 넘기지 않는 다른 소비자는 오늘과 같다):
  1. 준비: `reserve_run` → spawn(task는 `start_gate`를 기다리며 launcher를 아직 부르지 않음) → `attach_run_handle`. 이 시점에 run은 registry에 있고 취소할 수 있다.
  2. G 아래: 토큰이 `Pending`이면 `Registered{runId}`로 바꾸고 T-start를 A-turn으로 인계한다. 토큰이 `Cancelled`면 바꾸지 않는다.
  3. G 밖: `Registered`면 `start_gate`를 연다(launcher 실행). `Cancelled`면 registry에서 그 run을 취소한다. `start_gate`가 닫힌 채 drop되면 task는 launcher를 부르지 않고 끝난다.
- 선형화 지점은 2의 G 아래 전이다. 그 앞에서 온 취소는 `Pending→Cancelled`(실행 0), 그 뒤에서 온 취소는 registry의 실제 run을 취소한다.
- 시험(각각 결정적 gate): registry 예약 전 취소, spawn 뒤·attach 전 취소, attach 뒤·전이 전 취소, 전이 뒤·gate 열기 전 취소, 각 지점의 future abort. 모두 "취소가 성공이면 launcher·prompt 실행 0"과 "예약 해제 누락 0"을 단정한다.

### 알림 전달 예약 (Codex 재검토 3 F2)

- 오늘 자식 보고 도구는 결과를 저장하고 알림 전달기를 `tokio::spawn`한 뒤 곧바로 돌아간다(`agent_tools.rs:540-570`). 전달기가 처음 돌기 전에 자식 turn과 보고 호출이 끝나면, 예약만으로 세는 활동이 0이 되어 wait-stop이 `stopping`으로 넘어갈 수 있다. 그 뒤 전달기의 `send_and_wait`는 2'로 거절되어 알림을 잃는다.
- 수정:
  - 저장된 미전달 알림(대상 coordinator run이 살아 있음)을 활동에 센다(저장소 파생). 알림은 보고 호출이 돌아가기 전에 저장되므로 공백이 없다.
  - 보고 도구는 C-call을 놓기 전에 N-notify 예약을 만들어 전달기로 넘긴다. N-notify는 결과 저장 transaction commit까지 유지한다. `send_and_wait`는 그 안에서 별도의 A-turn을 잡고 prompt 실행이 끝나면 놓는다(인계하지 않음).
  - 비우기에 들어갈 때와 전달 실패 뒤(재시도 가능 실패), 서버가 알림 전달 한 바퀴를 스스로 돈다(backoff). 저장된 미전달 알림이 외부 계기 없이 남아 wait를 영원히 막지 않게 한다.
  - **중단된 전달의 회수(Codex 재검토 4 G1)**: 전달기는 전달 전에 `Dispatching`을 저장하고, 다음 전달은 `Pending`·재시도 가능 `Failed`만 고른다(`notification_dispatcher.rs`). 그래서 `Dispatching` 저장 뒤 future가 drop되거나 결과 저장이 실패하면, 그 알림은 활동으로 남는데 재시도에서는 고르지 않아 wait가 끝나지 않는다(`recover_interrupted`는 재시작 복구 경로라 여기서 돌지 않는다).
    - 전달 시도마다 `attemptId`를 발급해 `Dispatching{attemptId}`로 저장한다. 그 시도의 N-notify 예약이 **결과 저장 commit까지** G의 활동 표에 있다(Codex 재검토 5 G2: A-turn으로 인계하면 prompt 완료 뒤·결과 저장 전에 예약 없는 구간이 생겨, 회수가 정상 시도를 되돌리고 coordinator turn을 한 번 더 만든다).
    - 예약 guard가 결과 저장 없이 해제되면(drop·결과 저장 실패), guard가 회수를 예약한다(6''). 서버의 전달 한 바퀴도 시작할 때 "예약이 없는 `attemptId`의 `Dispatching`"을 회수한다.
    - 회수는 같은 `attemptId`일 때만 `Failed(retryable)`로 되돌린다. 살아 있는 시도는 예약이 있어 회수하지 않는다.
  - 시험: `Dispatching` 저장 직후 abort, 결과 저장 실패 주입 → 외부 복구 호출 없이 재전달 → wait 종료. 정상 전달 중에 회수 한 바퀴를 돌려도 그 시도가 되돌려지지 않음. **`send_and_wait` 반환 뒤·결과 transaction 직전 gate에서 회수 → 상태 변경·재전달 0. 같은 지점에서 abort → 회수되어 재전달 1회.**
- 시험: 전달기 첫 poll을 gate로 막은 채 보고 호출과 자식 turn을 끝내고 wait-stop 요청 → 멈추지 않음 → gate 해제 → 알림 전달 → 멈춤. 재시도 가능 실패 주입 → 서버가 다시 전달 → 멈춤.

### 상태 전이 표

| # | 사건 | G 아래 조건 | G 아래 효과 | G 밖 동작 | 성공 | 오류 | 취소·abort |
|---|---|---|---|---|---|---|---|
| 1 | 새 작업 호출(N) | 상태 `serving` | C-call 예약 | 호출 처리(→ 필요 시 2·3·4) | 해제 | 해제 | 해제 |
| 1' | 새 작업 호출(N) | 상태 `draining`/`stopping` | 없음 | `draining` 거절 / 503 | — | — | — |
| 2 | prompt 실행 시작(엔진 진입점) | 상태 ≠ `stopping` | A-turn 예약 | 세션 실행 future(초기 순서·대기열 차례 기다림 포함) | future 끝에서 해제 | future 끝에서 해제(RPC 오류 포함) | drop으로 해제 |
| 2' | prompt 실행 시작 | 상태 `stopping` | 없음 | 실행하지 않음(내부 경로는 run 취소로 처리) | — | — | — |
| 3 | 교환 전달(`sendPrompt` + continuation) | K 조건 모두 참(R7) + 미소비 + 상태 ≠ `stopping` | 소비 표시 + X-deliver 예약 | 엔진 대기열 등록 | 등록 성공 → A-turn으로 인계(X 해제는 인계와 원자적으로) | 등록 실패(run 없음) → X 해제, 소비는 유지, `failedExchangeDeliveries` 기록 | drop → X 해제, 소비 유지 + 실패 기록 |
| 3' | 같은 교환으로 둘째 전달 | 소비 표시 있음 | 없음 | 같은 키면 기존 멱등 결과, 다른 키면 N 거절 | — | — | — |
| 4 | 대기 task 배정 | 상태 ≠ `stopping` + (비우기 중이면 비우기 전 생성) | 기동 토큰 `Pending` 만들기 + T-start 예약 | 저장소 RMW: `Ready`·예약 없음 → `Starting{token}`(아니면 기존 예약 반환하고 T 해제) → 엔진 **준비**(fingerprint, registry 예약, 장벽에서 기다리는 spawn, attach) | G 아래 `Pending→Registered{runId}` + T→A 인계 → G 밖에서 시작 장벽 열기 | 준비 실패 → 토큰 `Failed`, T 해제, task는 오늘 규칙의 실패 상태 | drop → 토큰 `Failed`, 준비한 run이 있으면 registry에서 취소, T 해제 |
| 5 | task 취소(배정 전후) | 토큰 상태 | `Pending`이면 `Cancelled`로 바꾼다. `Registered`면 run id를 넘긴다 | `Pending→Cancelled`: 시작 장벽은 열리지 않음(실행 0), task `Cancelled`. `Registered`: registry의 실제 run 취소 | — | — | — |
| 5' | 기동 경로가 G 아래 전이를 하려는 순간 | 토큰이 `Cancelled` | 전이하지 않음 | 준비한 run을 registry에서 취소(장벽 닫힌 채 drop → launcher 실행 0), T 해제 | — | — | — |
| 6' | 자식 보고가 coordinator 알림 저장 | 상태 ≠ `stopping` | N-notify 예약(보고 C-call 해제 전, 전달 시도 id 발급) | 전달기로 넘김 → 알림을 `Dispatching{attemptId}`로 저장 → `send_and_wait`(그 안에서 A-turn을 따로 잡고 놓음) → 결과 저장 transaction | **결과 저장 commit 뒤에만** N-notify 해제 | 재시도 가능 실패: `Failed(retryable)`로 저장, 서버가 backoff로 다시 전달. **결과 저장 자체가 실패**하면 6''로 회수 | drop → 6''로 회수 후 해제 |
| 6'' | 전달 시도 회수 | 알림이 `Dispatching{attemptId}`이고 그 `attemptId`의 N-notify 예약이 G에 없음(A-turn 유무는 보지 않음) | — | 저장소 RMW로 같은 `attemptId`일 때만 `Failed(retryable)`로 되돌리고 서버 재전달 예약. 살아 있는 시도의 `Dispatching`은 건드리지 않음 | — | — | — |
| 6 | 자식 run 바인딩(`bind_child_run`) | task가 `Cancelled`면 거절 | — | — | — | — | — |
| 7 | 정지 판정(wait·idle) | 활동 작업 0(임대 조건 포함) | 상태 `stopping` | 받아들인 호출 drain → 쉬는 세션 취소 → 안내 파일 삭제 | — | — | — |
| 8 | 강제 정지·SIGTERM | 항상 | 상태 `stopping` 직전 `close_all_benches` 예약 | 작업대 닫기(모든 run 취소 → A-turn들이 drop으로 해제) → 7의 G 밖 동작 | — | — | — |

### 이 표가 닫는 실패 순서

| 실패 순서 | 닫는 칸 | 검증 |
|---|---|---|
| E1: 대기열 prompt만 남은 순간 활동 0으로 보여 멈춤 | 2(대기열 차례를 기다리는 동안도 A-turn) | 이전 완료와 다음 전송 사이에서 wait-stop 요청 → 멈추지 않음 |
| E1: RPC 오류 뒤 활동이 영구히 남음 | 2(future 끝에서 해제) | RPC 오류 주입 → 활동 0 |
| E1: 정지 판정과 동시 예약 | 2'·7(G 직렬화) | 판정·예약 교차 반복 → "멈춘 뒤 실행" 0 |
| E2: 즉시 전송이 바쁨으로 실패해 교환 유실 | 3(대기열 경로) | 알림 prompt와 경합하는 실제 경로 |
| C5: 한 교환으로 prompt 반복 | 3' | 다른 키·동시 요청 |
| E3: 동시 배정으로 run 둘 | 4(RMW 비교 후 변경) | 서로 다른 키 동시 배정 100회 |
| E4: 예약 뒤·등록 전 취소가 성공했는데 run이 뜸 | 5·5'·6(토큰 인계) | 엔진 등록 직전 gate로 멈춤 → 취소 완료 → gate 해제 → prompt 실행 0, task `Cancelled` |
| E4: 등록이 먼저면 취소가 실제 run을 멈춤 | 5(`Registered`) | 등록 뒤 취소 → run 취소 확인 |
| F1: 비동기 등록 중간의 취소(예약 전·spawn 뒤·attach 전·전이 전·장벽 전) | 4·5·5'(시작 장벽 + G 아래 선형화) | 지점별 gate 시험, abort 포함 |
| F2: 보고 반환 뒤 전달기 첫 poll 전에 활동 0 | 6'(저장소 파생 + N-notify) | 전달기 첫 poll gate 시험, 재시도 실패 주입 |
| G1: 중단된 `Dispatching` 알림이 영구히 남아 wait를 막음 | 6''(시도 소유권 회수) | `Dispatching` 저장 직후 abort, 결과 저장 실패 주입, 정상 시도 비회수 |
| G2: prompt 완료 뒤·결과 저장 전의 정상 시도를 회수해 중복 전달 | 6'(N-notify를 결과 commit까지 A-turn과 별개로 유지) | 결과 transaction 직전 gate에서 회수 → 변경 0, 같은 지점 abort → 회수 |

### 데스크톱 없이 실행을 유지한다는 목표와의 관계

- 서버가 소유하는 것: A-turn(엔진 실행과 엔진 대기열), T-start(시작 장벽 포함), 알림 전달(N-notify, 저장된 미전달 알림의 서버 재시도), 대기 자식 명령. 데스크톱이 없어도 이미 시작한 turn·대기열 prompt·orchestration 알림은 끝까지 간다.
- **아직 데스크톱 UI에 의존하는 것**: 교환 요청의 라우팅·패널 대기열(043). 교환 요청은 창의 원장이 받아 대상 패널로 라우팅하고, 패널 대기열이 `run.sendPrompt`를 부른다. 데스크톱이 없으면 새 교환은 전달되지 않는다(서버는 `undeliverableExchanges`로 보고하고 활동에 세지 않는다).
- 따라서 "교환 전달의 서버 소유(서버가 `send`/`queue` 교환을 대상 run 엔진 대기열에 직접 넣기)"는 **044 밖**이다. 후속 미완료 표(plan)에 두고 5단계 (a) 완료로 세지 않는다.

## R8. 앱 전체 종료 대 창 닫기 — 실제 종료 이벤트 순서 (사용자 검토 2·4)

- **사실(현재 코드·042 기록)**:
  - 042 스모크에서 macOS 앱 메뉴 Quit(`PredefinedMenuItem::quit` → NSApp terminate)은 `ExitRequested` 없이 `RunEvent::Exit`만 왔다(`lib.rs:374`).
  - 오늘 코드는 `Destroyed`마다 작업대를 닫는다(`window_lifecycle.rs`).
  - 종료 경로(`ExitRequested`·`Exit`)는 `close_all_benches`를 부른다(`lib.rs:393·414`).
  - 따라서 오늘은 앱 종료가 모든 run을 끝낸다. 044는 두 곳을 모두 바꿔야 한다.
- **아직 모르는 것**: 종료 경로마다 창 이벤트(`CloseRequested`·`Destroyed`)가 오는지, 오면 종료 신호보다 먼저인지 뒤인지. **이 순서를 확인하기 전에는 어떤 판정 규칙도 순서에 무관하다고 주장하지 않는다.**
- **R8-spike(구현 전 필수, 결과를 이 절에 기록)**: 실제 앱(외부 서버 모드 빌드)에서 다음 종료 경로의 창·앱 이벤트 순서를 로그로 남긴다.
  - (a) 빨간 버튼
  - (b) Cmd+W(`close_window` 메뉴)
  - (c) 앱 메뉴 Quit(Cmd+Q)
  - (d) Dock 메뉴 Quit
  - (e) AppleScript `quit`
  - (f) 마지막 창 닫기
  - (g) `SIGTERM`
  - (h) 로그아웃·재시동(자동화가 어려우면 관측 불가로 기록)
- **R8-spike 결과 (T002, 2026-09-28, macOS Apple Silicon, debug 번들 `AW Spike 044.app`, 043 코드 + 이벤트 로거)**. System Events·AppleScript로 자동화했고, 창 두 개(main + Settings)에서 실행했다. 로그는 세션 scratchpad `044/spike/`.

  | 경로 | 관측한 순서 | 반복 |
  |---|---|---|
  | (a) 빨간 버튼(Settings, main 남음) | `settings CloseRequested` → `settings Destroyed` | 1 |
  | (b1) 메뉴 `Window > Close Window` 클릭(Settings) | `settings CloseRequested` → `settings Destroyed`(main 남음) | 2 |
  | (b2) Cmd+W 키 입력(System Events `keystroke`, Settings가 앞) | `settings CloseRequested` → `main CloseRequested` → 두 창 `Destroyed` → `ExitRequested` → `Exit`(**두 창 모두 닫힘**) | 3 + 가설 검사 2 |
  | (c) 앱 메뉴 Quit(Cmd+Q) | `run Exit`**만**(창 이벤트 없음) | 1 |
  | (d) Dock 메뉴 Quit | `run Exit`만 | 1 |
  | (e) AppleScript `quit` | `run Exit`만 | 1 |
  | (f) 마지막 창 빨간 버튼(main) | `main CloseRequested` → `main Destroyed` → `ExitRequested` → `Exit` | 1 |
  | (g) `SIGTERM` | 이벤트 없음(기본 처리로 프로세스 종료) | 1 |
  | (h) 로그아웃·재시동 | **관측 불가**(자동화하지 않음) | — |

  - (b2)의 원인은 확인하지 못했다. 파일·Window 메뉴의 `close_window` 중복을 의심해 하나를 뺀 빌드로 다시 해 봤지만 결과가 같아 기각했다. 화면에 Cmd+W 처리기도 없다(검색 0건). 메뉴 항목 클릭은 한 창만 닫는다. 사람이 누른 Cmd+W에서도 같은지는 T046에서 확인한다(미확인 위험).
  - (c)(d)(e)에서는 `Exit` 전에 창 이벤트가 오지 않았다. 따라서 **이 세 경로는 `CloseRequested`가 종료 신호보다 먼저 오지 않는다(관측)**. `Exit` 뒤에 창 이벤트가 오는지는 프로세스가 곧 끝나 로그에 없다. 종료 의도 표시가 `Exit`에서 서므로, 그 뒤의 `Destroyed`는 닫기 의도가 없어 작업대를 닫지 않는다.
  - (f)는 사용자가 마지막 창을 닫은 경우다. `CloseRequested`가 먼저 오므로 창 닫기(작업대 닫기)로 판정하고, 그다음 앱이 끝난다.
  - (g)는 앱 처리가 없다. 임대는 TTL로 거둬지고 작업대·run은 서버에 남는다. `close_all_benches`는 불리지 않는다.
- **설계 확정(T003)**:
  - 닫기 의도 = 그 창의 `CloseRequested`. 종료 의도 표시(`quitting`)가 선 뒤의 `CloseRequested`는 의도로 세지 않는다.
  - 종료 의도는 `ExitRequested`·`RunEvent::Exit`에서 선다. (c)(d)(e)는 `Exit`만 오고 그 전에 창 이벤트가 없으므로 이것으로 충분하다(관측).
  - `Destroyed`: 닫기 의도가 있으면 `retireWindow{closeBench:true}`, 없으면 `{closeBench:false}`.
  - 외부 서버 모드의 `ExitRequested`·`Exit`는 `close_all_benches`를 부르지 않는다. 임대만 푼다(상한 2초).
  - 검증 대상 종료 경로(T045): (c)·(d)·(e)·(g)는 run 지속. 대조(T046): (a)·(b1)·(f)는 그 창 작업대 닫힘, (b2)는 관측대로 두 창 작업대가 닫히는지 확인하고 위험 목록에 둔다.
- **설계 방향(spike 결과로 확정, 위 "설계 확정"이 우선)**:
  - 작업대 닫기는 "사용자가 그 창을 닫으려 했다"는 신호가 있을 때만 한다. 앱 종료가 창을 걷어 낼 때는 토큰·표만 폐기한다(`desktop.retireWindow{closeBench:false}`).
  - 앱 종료 신호를 가장 이르게 잡는 지점을 경로마다 정한다. 후보: 앱 메뉴 Quit을 직접 처리하는 메뉴 항목, `ExitRequested`, `RunEvent::Exit`, macOS terminate 알림에 해당하는 tao/Tauri 신호.
  - spike에서 어떤 종료 경로가 종료 신호보다 먼저 `CloseRequested`를 내면, 그 경로에서는 `CloseRequested`만으로 창 닫기를 판정할 수 없다. 그 경로는 종료 의도를 먼저 세우는 수단(직접 처리 메뉴 항목, 더 이른 신호)을 구현해야 한다.
  - 외부 서버 모드의 종료 경로는 `close_all_benches`를 부르지 않는다. 임대만 풀고(`lease.release`), 대기 중인 `retireWindow`를 짧은 상한(2초) 안에 흘려보낸 뒤 끝난다.
  - 판정은 순수 함수 `window_close_intent`로 뽑는다. **spike에서 관측한 순서 조합**을 시험으로 고정한다.
- **완료 조건(SC-001·SC-006)**:
  - spike에서 **관측한 종료 경로마다** 구현과 실제 검증을 한다. 실제 앱에서 run을 시작하고 그 경로로 종료한다. 프로세스가 실제로 없어졌는지(PID 소멸) 확인한 뒤, 소유자 클라이언트로 그 run이 진행 중이고 출력이 이어짐을 확인하고 취소한다.
  - 대조로 창 닫기((a)·(b))에서는 그 작업대의 run이 취소됨을 확인한다.
  - **위험 목록에 적는 것만으로는 완료가 아니다.** 관측한 경로 중 하나라도 구현·검증이 안 되면 SC-001·006은 미완료로 남는다.
  - 자동화할 수 없어 관측하지 못한 경로(예: 로그아웃)는 "관측 불가·미검증"으로 따로 적고 완료로 세지 않는다.

## R9. 임대와 유휴 종료

- **Decision**:
  - `lease.acquire{clientKind, clientId}` → `{leaseId, ttl}`, `lease.renew`, `lease.release`. 모두 소유자 전용이다.
  - 기본값: 임대 TTL 30초, 데스크톱은 10초마다 갱신. 서버 유휴 시간은 기본 10분이다(`--idle-timeout`; 시험은 짧게).
  - 유휴 판정: 유효 임대 0 + `ActiveWork` 0이 유휴 시간 동안 이어지면 `draining{idle}` → (새 호출 없음 확인) → `stopping`.
  - 유휴 비우기 중 임대가 새로 잡히면 `serving`으로 돌아간다. wait 비우기는 돌아가지 않는다.
  - **경합(Codex 구현 리뷰 high)**: 정지 판정은 활동 세대를 읽고 파생 값을 기다린 뒤 전이한다. 그 사이 임대가 비우기를 서빙으로 되돌리면 낡은 판정이 서빙을 `stopping`으로 바꿀 수 있었다.
    - 상태 전이(비우기 시작·유휴 비우기 취소)도 활동 세대를 바꾼다. 그래서 전이 전에 시작한 판정은 거절된다.
    - 비우기 정지 판정은 판정 시작 때 이미 서빙이면 멈추지 않는다.
    - 증거: 관문 수준의 끼어들기 시험(세대 읽기 → 서빙 복귀 → 판정)과 ServerControl 수준 시험(임대 뒤 판정). `derive` 도중의 실제 thread 끼어들기를 강제하는 시험은 아니다.
- **Rationale**: 앱이 강제로 죽으면 임대를 풀 수 없다. TTL로 결국 거둔다(FR-021).

## R10. 정지 요청

- **Decision**: `server.stop{mode: default|wait|force}`(소유자 전용).
  - `default`: `ActiveWork`가 있으면 `conflict`(blocker 목록 포함). 없으면 곧바로 `stopping`.
  - `wait`: `draining{wait}`로 들어가 `ActiveWork`가 0이 되면 `stopping`.
  - `force`: `close_all_benches`(run 취소·권한 대기 해제) → `stopping`.
  - `SIGTERM`·`SIGINT`는 `force`와 같다. 상한 30초 뒤 남은 호출을 버리고 끝낸다(경고 기록).
  - `server.status`: 상태, `ActiveWork` 내역, 임대 수, 인스턴스·세대·버전을 돌려준다.

## R11. compat·embedded 모드

- **Decision**:
  - 기본 모드는 외부 서버다. 데스크톱은 런타임을 관리 상태로 두지 않는다.
  - compat command는 등록돼 있어도 "외부 서버 모드에서는 쓸 수 없음" 오류를 돌려준다(런타임 상태를 꺼내다 panic하지 않게 `Option`으로).
  - 부팅이 서버를 찾거나 띄우지 못하면 **연결 실패 화면**을 보여 주고 다시 시도하게 한다. 호환 경로로 대체하지 않는다.
  - `AW_WORKBENCH_MODE=embedded`(개발·시험)만 043 경로를 쓴다. 그 경로는 host crate로 조립하고 같은 `owner.lock`을 잡는다. 잡지 못하면(외부 서버가 떠 있음) 부팅이 실패한다.
- **Rationale**: 외부 서버와 앱 안 런타임이 같은 데이터 디렉터리를 동시에 쓰면 단일 writer가 깨진다(FR-016·017).

## R12. #207: 닫힌 작업대의 멱등 기록 부활

- **사실(코드 판독)**:
  - `EpochIdempotency::run`(`epoch_idempotency.rs:127-188`)은 실행이 끝나 `Ok`이면 `record`를 부른다. `record`는 `scopes.entry(scope).or_default()`(:192)로 scope 표를 **다시 만든다**.
  - `drop_bench`(:209-213)는 작업대 닫기(`finish_close`, `bench_service.rs:277-279`)가 부른다. 그런데 `run.cancelAndSend` 같은 epoch 처리 호출은 작업대 입장권을 쥐지 않는다(`run_service.rs:170-193`). 그래서 닫기가 그 호출을 기다리지 않는다.
  - 결과: 닫기 뒤 늦게 성공한 호출이 닫힌 작업대 scope에 기록을 남긴다. 같은 키 재시도는 실행 전 조회에서 그 기록(`Complete`)을 돌려준다. 시험 `acp_permission_exit.rs:263`이 `notFound`를 기대하다 실패한 까닭이다(PR #206 CI).
- **Decision**:
  - `EpochIdempotency`에 세대 범위의 **닫힌 작업대 tombstone** 집합을 둔다. `drop_bench`가 tombstone을 세운다. `record`는 tombstone이 선 작업대 scope에 기록하지 않는다(결과는 호출자에게 그대로 돌려준다).
  - 실행 전 조회도 tombstone scope는 비어 있는 것으로 본다. 이후 handler가 작업대를 풀면 `notFound`가 된다.
  - tombstone은 세대 동안 유지한다. 작업대 id는 재사용되지 않는다(uuid).
- **검증**:
  - 결정적 재현 시험: 가짜 엔진의 prompt 완료를 문(gate)으로 붙잡는다 → `close_all_benches` → 문 해제 → 호출 `Ok` → 같은 키 재시도가 `notFound`.
  - 수정 전 코드에서 이 시험이 실패함(`Complete`)을 먼저 기록한다.
  - `acp_permission_exit` 원 시험을 반복 실행해 기록한다.
- **Alternatives**:
  - epoch 처리 호출이 입장권을 쥐어 닫기가 기다리게 함 → 권한 대기 중인 prompt를 닫기가 영원히 기다리는 교착 위험(닫기가 권한 대기를 푸는 쪽)이라 기각.
  - `drop_bench`가 진행 중 slot을 기다림 → 같은 이유로 기각.

## R13. 검증 도구

- 서버 통합 시험 host: `http_test_host` 예제를 host crate 조립으로 바꾼다. 가짜 엔진은 `test-hooks`로 유지한다.
- 실제 프로세스 시험: 서버 바이너리를 `std::process::Command`로 띄우는 Rust 통합 시험(`apps/agentic-workbench-server/tests/`). 대상은 동시 시작 10회, 강제 kill 뒤 복구, 안내 파일 권한, 유휴 종료, 정지 세 방식이다.
- 앱 스모크: 043 probe를 재사용한다. 외부 서버 모드로 개발·배포 출처를 실행한다. 새 시나리오 `quit`은 run 시작 → 앱 종료 → 스모크 스크립트가 소유자 클라이언트로 run 지속·출력·취소를 확인한다.

## 부록: Tauri 결합 지도 (요약, `cb0bd4c`)

| 조각 | 위치 | 044 처리 |
|---|---|---|
| 런타임 `bootstrap_with(DataPaths)` | `lib.rs:248-261` | host crate |
| `RuntimeAdapters` 중 `desktop`·`launch_decorator`(Tauri 브리지) | `lib.rs:258-259` | 외부 모드: no-op 브리지 + `McpLaunchDecorator` |
| MCP 서버 `McpServerState::start(AppHandle)` | `lib.rs:272`, `mcp/mod.rs` | host crate, 런타임 직접 주입 |
| HTTP `WorkbenchHttpState::start`(발급기·표·resolver·출처 정책) | `workbench_http.rs:124-178` | host crate, 소유자 resolver 추가 |
| 종료 `ExitGate`·`drain_for_exit`(`close_all_benches` 포함) | `lib.rs:376-426` | 외부 모드: 임대 해제만 |
| `window_principals`·`window_lifecycle`·`desktop_benches` | src-tauri | 데스크톱에 남음. 폐기·닫기는 서버 호출로 |
| compat command(`Caller{runtime, principal}`) | `tauri_commands.rs`, `workbench_compat.rs` | 외부 모드: 쓸 수 없음 오류 |
| 네이티브 삽입(`dispatch_script`) | `tauri_desktop_bridge.rs` | 외부 모드: 없음. 제목은 `apply_window_title` |
| appearance·layout·창 상태 저장소 | src-tauri | 데스크톱에 남음 |
