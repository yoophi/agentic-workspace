# Workbench

AW를 독립 서버 + 얇은 클라이언트로 전환하기 위해 만든 컨텍스트. 데스크톱 셸·테스트 하네스·앞으로의 CLI가 **같은 계약**으로 호출하는 operation과, 서버가 소유하는 상태의 언어를 정의한다.

## Language

### 호출 계약

**Workbench**:
모든 클라이언트가 같은 계약으로 호출하는 단일 인터페이스. 호출자가 데스크톱인지 테스트인지에 따라 결과가 달라지지 않는다.
_Avoid_: 서버, 백엔드, runtime

**Operation**:
`<도메인>.<동사>` 이름을 가진 호출 단위. 조회 아니면 변경 중 하나다.
_Avoid_: command(Tauri command와 혼동), endpoint, route

**조회 (Query)**:
상태를 바꾸지 않는 operation. 멱등성 키를 요구하지 않는다.
_Avoid_: read, fetch

**변경 (Command)**:
상태나 외부 부작용을 일으키는 operation. 멱등성 키가 필수다.
_Avoid_: mutation, write, Tauri command

**Principal**:
인증 계층이 부여한 호출자의 정체와 권한 범위. 호출 입력으로 지정할 수 없다.
_Avoid_: user, caller, client

**창 주체 (Window Principal)**:
데스크톱 창 하나에 묶인 principal. 창을 만들 때마다 새 **incarnation**으로 발급되므로, 같은 창 이름으로 다시 연 창도 다른 주체다. 창이 닫히면 그 주체의 자격 증명은 모두 무효가 된다.
_Avoid_: desktop user, window label(주체 이름으로는)

**소유자 주체 (Owner Principal)**:
같은 OS 사용자가 데이터 디렉터리 안내 파일의 자격 증명으로 얻는 principal. 모든 작업대를 조회·구독·취소할 수 있어, 데스크톱 없이도 서버의 run을 다룬다. agent만 부를 수 있는 operation은 부르지 못한다.
_Avoid_: admin, root, desktop(창 주체와 혼동)

**Scope**:
principal에게 허용된 도메인별 조회/변경 권한 단위.
_Avoid_: permission(agent 실행 권한과 혼동), role

**Request ID**:
호출 시도 하나를 추적하는 식별자. 재시도마다 새로 만든다.
_Avoid_: trace id, call id

**멱등성 키 (Idempotency Key)**:
같은 변경의 재시도를 하나로 묶는 식별자. 재시도에만 재사용한다.
_Avoid_: request id, dedupe key

**Fault**:
operation 실패 응답. 안정적 오류 코드, 재시도 가능 여부, 적용 여부, 사람이 읽는 한 문장 message를 가진다.
_Avoid_: error string, exception

**적용 여부 (Outcome)**:
실패 응답이 알려 주는 부작용 상태 — `적용 안 됨`, `적용됨`, `불명`.
_Avoid_: status, result

### 저장과 변경 기록

**저장 단위 (Aggregate)**:
한 번에 하나의 변경만 적용되고 자기 revision을 가지는 상태 묶음.
_Avoid_: store, table, collection

**Revision**:
저장 단위의 누적 변경 횟수. 변경마다 1 증가하며 기대 revision 검사에 쓰인다.
_Avoid_: version, etag

**변경 기록 (Operation Ledger)**:
변경의 의도와 결과를 부작용 전후에 남기는 내구 기록. 상태는 `대기`·`적용됨`·`실패`·`불명`.
_Avoid_: log, journal(run 이벤트 journal과 혼동), outbox

**재시작 판정 (Reconciliation)**:
재시작 시 미확정 변경 기록을 자동 재실행 없이 `적용됨` 또는 `불명`으로 판정하는 절차.
_Avoid_: recovery(손상 파일 복구와 혼동), replay

**종료 상태 규칙 (End-State Rule)**:
외부 부작용(Git Worktree 생성·삭제)의 재시작 판정 규칙. 원하는 종료 상태가 관찰되면 누가 만들었는지 구별하지 않고 `적용됨`으로 보고, 아니면 `불명`이다.
_Avoid_: 존재 확인, idempotent check

### 이벤트

**이벤트 스트림 (Event Stream)**:
순서가 보장되는 이벤트의 단위. 한 run, 한 Worktree처럼 대상 하나에 대응한다. 서로 다른 스트림 사이의 순서는 약속하지 않는다.
_Avoid_: topic, channel, event name

**세대 (Server Epoch)**:
서버 기동 한 번을 가리키는 표식. 세대가 다르면 순번을 비교하지 않는다.
_Avoid_: session, instance id, boot id

**Cursor**:
호출자가 마지막으로 반영한 스트림·세대·순번. 구독은 cursor 다음부터 이어 받는다.
_Avoid_: offset, checkpoint, last event id

**Gap**:
cursor 다음을 이어 붙일 수 없다는 신호(보관 범위 밖, 세대 다름, 구독자가 너무 느림). 받은 쪽은 상태를 다시 조회해 동기화한다.
_Avoid_: error, lag(원인 하나일 뿐), missing events

**상태 복원용 이벤트 (State Event)**:
순서대로 적용하면 대상의 상태가 복원되는 이벤트. 보관되고 replay된다. run 이벤트가 이에 해당한다.
_Avoid_: persistent event, durable event(메모리에만 보관된다)

**알림용 이벤트 (Notification Event)**:
"다시 조회하라"는 신호일 뿐 본문이 상태가 아닌 이벤트. 보관·replay하지 않는다. Worktree 변경이 이에 해당한다.
_Avoid_: diagnostic event, ephemeral event

**실행 정보 유실 (Runtime Lost)**:
서버 재시작 등으로 run의 이벤트를 더는 이어 받을 수 없게 된 상태. run이 실패했다는 뜻은 아니다.
_Avoid_: crashed, failed, disconnected

**Client Instance**:
이벤트 구독을 여는 주체 하나(데스크톱 창 하나, HTTP 연결 하나). 이벤트를 누구에게 보낼지만 정하며 run이나 Worktree를 소유하지 않는다. 소유는 작업대(Bench)의 일이다.
_Avoid_: window, client, session(Provider Session과 혼동)

### 서버 생명주기

**서버 인스턴스 (Server Instance)**:
한 데이터 디렉터리를 쓰는 유일한 Workbench 서버 프로세스 하나. 데이터 디렉터리마다 동시에 하나만 있다. 클라이언트는 인스턴스 식별자로 자기가 붙은 서버가 맞는지 확인한다.
_Avoid_: daemon, backend, host

**임대 (Lease)**:
붙어 있는 클라이언트가 만료 시간을 두고 갱신하는 "아직 쓰고 있음" 표시. 임대가 있는 동안 서버는 유휴 종료하지 않는다. 임대는 작업이 아니다. 작업을 끝낼 때까지 서버를 붙잡지 않는다.
_Avoid_: session, connection, keepalive

**비우기 (Draining)**:
정지를 요청받은 서버가 새 작업을 거절하면서 이미 받아들인 작업이 끝나기를 기다리는 상태.
_Avoid_: shutdown, graceful stop

**비우기 분류 (Drain Class)**:
비우는 동안 operation을 받을지 정하는 분류. 조회, 끝내는 제어(권한 응답·취소·보고처럼 활동 작업을 줄이는 것), 조건부 이어 가기(이미 약속된 작업을 마저 하는 것), 새 작업(거절) 네 가지다.
_Avoid_: allowlist, priority

**활동 작업 (Active Work)**:
정지·유휴 종료를 막는 일. 진행 중 turn·권한 대기, 배정된 task, 배정할 쪽이 있는 대기 task, 전달할 알림, 판정 전 변경이 여기에 든다. 쉬고 있는 세션과 결과를 끝내 알 수 없는 변경은 들지 않는다.
_Avoid_: busy, running, pending

**보류 task (Deferred Task)**:
배정할 쪽(turn 중이거나 전달할 알림이 남은 coordinator)이 없어 정지를 막지 않는 대기 task. 사라지지 않고 저장돼, 서버를 다시 띄운 뒤 복구해 배정할 수 있다.
_Avoid_: dropped task, orphan

**작업 관문 (Work Gate)**:
활동 작업의 예약과 정지 판정을 한 줄로 세우는 관문. 정지가 결정된 뒤에는 새 예약이 서지 않고, 예약이 선 뒤에는 정지가 그것을 건너뛰지 않는다.
_Avoid_: lock, semaphore, mutex

### 도메인

**Project**:
사용자가 AW에 등록한 저장소 루트와 그 이름.
_Avoid_: repo, workspace

**Saved Prompt**:
사용자가 저장해 두고 다시 쓰는 프롬프트(label + 본문).
_Avoid_: template, snippet, prompt(실행 중 보내는 프롬프트와 혼동)

**Goal**:
Worktree 하나에 붙는 목표와 그 진행(토큰·시간) 기록.
_Avoid_: thread goal, objective(필드 이름)

**Agent Run Settings**:
Worktree별 agent 실행 설정(agent, 권한 모드, 모델, 명령 재정의).
_Avoid_: run config, preferences(외관 설정과 혼동)

**Worktree**:
세션이 작업하는 체크아웃 디렉터리. 저장소 루트일 수도, Git worktree일 수도 있다. 이를 가리키는 절대 경로 입력 필드 이름은 `workingDirectory`다.
_Avoid_: workspace, working directory(개념 이름으로는)

**작업대 (Bench)**:
호출자 하나가 Worktree 하나를 대상으로 연 작업 단위. run과 교환 작업 영역을 소유하고, 명시적으로 끝낼 때까지 유지된다. 한 Worktree에 여러 개를 동시에 열 수 있다. 데스크톱에서는 세션 창의 창 주체가 작업대 하나를 열고, 다른 창 주체는 그 작업대를 쓸 수 없다.
_Avoid_: window, session, workspace, owner(일반 명사로는)

**Git Worktree**:
`git worktree add`로 저장소에 붙인 추가 체크아웃. Worktree의 한 종류다.
_Avoid_: linked worktree, branch checkout

**Agent**:
AW가 실행할 수 있는 코딩 에이전트의 catalog 항목.
_Avoid_: provider(세션 파일을 남긴 주체를 가리킬 때만), model

**Provider Session**:
agent가 자기 형식으로 로컬에 남긴 과거 대화 기록. AW의 run이 아니며 AW가 만들지 않는다.
_Avoid_: agent session, ACP session, run

**데스크톱 표현 상태 (Desktop Presentation State)**:
글꼴 크기, 창 위치·크기, panel layout처럼 데스크톱 클라이언트 하나에만 속하는 설정. Workbench 밖이다.
_Avoid_: settings, preferences
