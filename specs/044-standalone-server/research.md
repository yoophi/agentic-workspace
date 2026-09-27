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
  - 클라이언트 `ensure(data_dir, exe)`:
    1. `startup.lock` 획득(상한 대기, 기본 20초).
    2. `server.json`이 있으면 끝점에 버전 확인과 **소유자 인증 상태 조회**를 한다. 인스턴스 식별자가 일치하고 준비 상태이면 붙는다.
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
  - 안내 파일에 32바이트 무작위 **소유자 자격 증명**을 넣는다(파일 0600).
  - 이 자격 증명의 주체는 새 `PrincipalKind::Owner`(`local:owner`)다. 가진 권한은 모든 scope와 `server:admin`, 그리고 **작업대 소유 판정 우회**(모든 작업대 조회·구독·닫기·run 취소)다.
  - 창 토큰은 소유자가 owner 전용 operation `desktop.issueWindowToken`으로 받는다. 입력은 `{label, incarnation, origin}`이고, 출력 `{token, expiresAt}`은 043과 같은 창 주체·출처 묶음이다.
  - 창 폐기는 `desktop.retireWindow {label, incarnation, closeBench}`다. 한 호출로 그 주체의 토큰·표 폐기와 (요청 시) 그 주체가 연 작업대 닫기를 한다.
  - 서버의 창 주체 등록은 두지 않는다. incarnation은 데스크톱이 발급하고, 서버는 소유자가 요청한 주체로 토큰을 만든다.
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
- **K가 필요한 실제 경로 — 교환 전달(사용자 검토 3)**:
  - 교환은 서버가 agent에 직접 보내지 않는다. 대상 창의 원장이 요청 이벤트를 받아 대상 run에 `run.sendPrompt`로 전달하고(키 `exchange-delivery:<requestId>`, 043), 그 뒤 `exchange.acknowledge`로 확인한다.
  - 따라서 확인(C)만 허용하면, 비우기 전에 요청된 교환은 전달되지 못한 채 남아 wait-stop이 끝나지 않는다.
  - 계약:
    - `run.sendPrompt` 입력에 선택 필드 `continuation: { exchangeRequestId }`를 더한다.
    - 서버는 다음을 모두 확인하면 `draining` 중에도 받는다. 하나라도 어긋나면 N으로 거절한다:
      - 그 교환이 호출자 작업대에 있다.
      - 상태가 확인 전(`accepted`)이다.
      - 대상 run이 이 run이다.
    - 화면의 교환 원장은 전달할 때 이 필드를 싣는다(오늘 키와 함께).
- **orchestration 후속 경로(사용자 검토 3)**:
  - coordinator 알림 전달은 서버 내부의 알림 전달기(`notification_dispatcher`)가 한다. 호출이 아니라서 입구 판정을 받지 않는다.
  - 화면의 `orchestration.dispatchPrompt`·`sendChildCommand`는 사용자가 새로 보내는 prompt다(N).
  - 대기 중인 자식 명령(`delivery: queue`)을 자식이 쉴 때 서버가 스스로 넘기는지, 아니면 클라이언트 호출이 필요한지는 **구현 전 확인 항목(R7-check)**이다.
    - 클라이언트 호출이 필요하면 그 호출을 K로 분류하고, 대기 항목 id를 입력으로 확인한다.
    - 확인 결과와 근거(코드 위치)를 이 절에 적는다.
- **활성 작업(`ActiveWork`)**:
  - 진행·예약 run 수
  - 권한 대기 수
  - 진행 중 orchestration task 수
  - **확인 전 교환 중 대상 run이 살아 있는 것**의 수
  - 이 프로세스가 적용 중인 ledger `pending` 수
  - 받아들인 분리 호출 수(HTTP·MCP)
  - 유효 임대 수(유휴 판정에만)
- **ledger `unknown`의 의미**:
  - `unknown`은 이전 세대에서 시작 복구가 적용 여부를 판정하지 못한 기록이다. 시간이 지나도 스스로 풀리지 않는다.
  - 그래서 `unknown`은 **활성 작업이 아니다**. wait·idle을 막지 않는다. `server.status`의 `unresolvedOperations`로 보여 준다.
  - 설계 문서는 "durable pending/unknown이 있으면 idle shutdown 불가"라고 적었다. 영구 `unknown`이 서버를 영원히 살려 두지 않도록 이 증분은 이렇게 **다르게 정하고 기록한다**. 이 프로세스가 적용 중인 `pending`만 막는다.
- **wait-stop이 끝남을 보장하는 논증**:
  - 새 run turn을 시작하는 호출은 K 조건을 만족하는 교환 전달을 빼면 모두 N이다: run.start, sendPrompt(조건 밖), steer, cancelAndSend, orchestration.dispatchPrompt·sendChildCommand·sendChildMessage·delegateGoal·createChildTask·retry·reassign·recover.
  - K는 비우기 시작 때 이미 있던 교환에만 해당한다. 새 교환 요청(`exchange.send`·`sendFromRun`)은 N이라 늘지 않는다. 따라서 K로 생기는 turn 수는 유한하다.
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
     - (a) 권한 대기 run: 권한 응답 → turn 완료 → 멈춤
     - (b) 확인 전 교환: 043 `createNetworkEvents` + `createExchangeReconciler`가 요청 이벤트로 라우팅 → `run.sendPrompt(continuation)` → 확인 → 교환 종결 → 멈춤
     - (c) 진행 중 orchestration 자식 task: 자식의 보고·결과(MCP 경로) → 알림 전달기가 coordinator에 전달 → task 종결 → 멈춤
     - (d) 대기 중 자식 명령: R7-check 결과에 따른 실제 전달 경로
  4. 각 실제 경로 시험의 대조 변이: 해당 C·K를 N으로 바꾸면 wait-stop이 끝나지 않는다(정해진 상한 안에 멈추지 않음을 단정).
  5. N operation이 `draining`으로 거절되고 효과가 없는 시험.
  6. ledger `unknown`만 남은 서버가 wait·idle에서 멈추고 `unresolvedOperations`에 보이는 시험.

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
- **설계 방향(spike 결과로 확정)**:
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
