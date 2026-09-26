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
