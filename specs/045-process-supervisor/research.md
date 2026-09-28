# Research: 서버 자식 프로세스 감독

## R1. 공통 경계와 owner 순서

**Decision**: `crates/process-supervisor`가 OS child handle, containment, output drain, cancel/wait를 소유한다. durable business execution은 기존 domain store/ledger에 owner와 attempt를 먼저 예약한다. read-only helper의 domain 의미는 transient지만 모든 ServerOwned child는 supervisor store에 durable containment recovery anchor를 가진다. supervisor가 `UnpublishedProcessLease`를 반환한 뒤 publication CAS와 outbox commit이 끝나야 accepted/started를 공개한다.

**Rationale**: 현재 `crates/acp-agent-core/src/infrastructure/acp/runner.rs:135-160`은 `LifecycleStatus::Started`를 emit한 뒤 `Command::spawn()`한다. spawn 실패에도 외부 projection이 started를 본다. 공통 crate가 durable store를 알게 하면 workbench/acp 의존이 역전되므로 reserve/commit은 consumer, 실제 child ownership은 supervisor로 나눈다.

**Alternatives considered**:
- runner에서 emit 한 줄만 아래로 이동: terminal·Git·helper와 future cancellation에는 같은 결함이 남는다.
- supervisor가 durable store까지 소유: 공통 crate가 workbench domain과 persistence에 결합된다.

## R2. cancellation-safe spawn/adopt

**Decision**: registry가 child를 소유하고 caller는 `#[must_use] UnpublishedProcessLease`만 가진다. lease 반환 뒤 publication caller와 resolver는 모두 `Adopted` 행의 CAS를 사용한다. caller는 한 transaction에서 `Adopted → Published`, domain result와 attempt-keyed outbox를 commit한다. resolver는 `Adopted → Aborting` CAS를 이긴 뒤에만 종료한다. transient helper는 `Adopted → Active`로 바꾸고 외부 event를 만들지 않는다. lease가 commit 전/후 어느 구간에서 drop되거나 ack response가 유실되면 registry는 response send 실패만으로 추정하지 않고 winner state를 resolve한다. 저장소 결과가 ambiguous하면 child를 containment 안에 quarantine하고 readiness를 내린 채 reconcile하며, 확인 없이 published child를 죽이지 않는다.

**Rationale**: Tokio `Child`는 기본적으로 handle drop이 process cancel을 뜻하지 않는다. `kill_on_drop`도 Unix reap 시점을 보장하지 않는다. 또한 Adopted lease가 caller에 전달된 뒤 durable commit과 ack 사이에는 response receiver drop만으로 알 수 없는 ownership handoff 구간이 있다. durable publication state를 정본으로 재판정해야 commit된 실행을 잘못 죽이거나 commit 안 된 실행을 남기지 않는다. [Tokio Command 문서](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)

**Alternatives considered**:
- `kill_on_drop(true)`만 사용: descendant tree와 deterministic reap을 보장하지 못한다.
- raw `Child`를 adapter에 반환: adapter마다 kill/wait race가 다시 생긴다.

**Cancellation fixtures**:
- adopt response를 보내기 전 caller drop: process cleanup, durable Reserved→Aborted.
- lease 수신 뒤 durable commit 전 drop: resolver가 Reserved를 보고 cleanup.
- durable Published commit 뒤 ack 전 drop: resolver가 Published를 보고 process 유지, started snapshot/retry는 같은 attempt.
- ack 처리 뒤 reply 전 drop: Published process 유지, 재시도는 같은 attempt/result.
- resolver storage fault와 future cancellation: bounded reconcile 뒤 정확히 하나의 keep 또는 cleanup outcome.
- publication CAS pause 뒤 cleanup CAS와 cleanup CAS pause 뒤 publication CAS: 최초 transition만 승리하고 loser는 같은 typed result를 재현.
- Published/outbox transaction 전후와 dispatcher send/ack 전후 crash: attempt별 durable logical event와 client projection 적용 정확히 1회.

## R3. Unix containment

**Decision**: macOS/Linux에서 composition-root executable의 내부 `__process-keeper` mode를 사용하되 process group을 전체 containment로 간주하지 않는다. keeper는 256-bit attempt nonce를 payload launch environment에 넣고 payload PID/start identity와 nonce를 handshake한 뒤에만 Adopted를 알린다. keeper는 target process inventory에서 같은 uid, 같은 nonce, launch 이후 start identity인 live process 집합을 유지한다. leader가 먼저 끝나거나 descendant가 새 group/session을 만들고 double-fork로 reparent돼도 nonce/start identity로 집합에 남는다. control pipe EOF 또는 terminate command에서 먼저 원 process group에 signal하고, 이어 identity 집합의 각 process를 start identity 재검증 후 종료한다. live set이 연속된 quiescence window 동안 0이고 direct payload/keeper wait가 끝나야 cleanup complete다.

**Rationale**: process group은 빠른 signal 단위일 뿐 `setsid`, 새 group과 double-fork를 막지 못한다. macOS `EVFILT_PROC NOTE_TRACK/NOTE_CHILD`는 SDK에서 10.5 이후 미지원으로 표시돼 자동 fork tree 추적 근거로 사용할 수 없다. Linux parent-death signal도 생성한 parent thread 기준이며 fork 자식에 상속되지 않는다. 따라서 leader/ppid/group이 아니라 attempt identity를 독립적으로 검증해야 한다. [Apple setpgid(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setpgid.2.html), [Linux PR_SET_PDEATHSIG](https://www.man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)

**Alternatives considered**:
- process group만 저장하고 다음 startup에 kill: leader 조기 종료, session/group 이탈과 PID/PGID 재사용에서 안전한 소유 증명이 없다.
- Linux PDEATHSIG만 사용: macOS와 의미가 다르고 descendant 전체 보장이 아니다.
- 별도 keeper binary: signed sidecar/packaging 항목을 늘린다. 같은 binary의 내부 mode면 새 artifact가 없다.

**Validation gate**:
- fixture는 leader 즉시 종료, `setsid`, 새 process group, double-fork+reparent, inherited control FD close 뒤 장기 실행을 각각 수행한다.
- keeper/parent hard kill 뒤 nonce+start identity live set, 원 process group, direct wait 상태를 모두 기록한다.
- PID 재사용 대조 또는 같은 숫자 PID를 흉내 낸 unrelated process는 nonce/start identity 불일치로 생존해야 한다.
- target process inventory가 live descendant nonce를 읽고 identity를 재검증하는 것을 실제 macOS/Linux에서 입증하지 못하면 group kill 성공만으로 통과하지 않는다. supervisor capability를 fail-closed로 두고 045 완료를 막는다.

## R4. Windows containment

**Decision**: payload를 `CREATE_SUSPENDED`로 만들고, breakaway를 허용하지 않는 Job Object에 assign하며 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`를 설정한 뒤 primary thread를 resume한다. assign/resume 전 실패는 payload를 종료·wait하고 Adopted를 반환하지 않는다.

**Rationale**: Job Object는 process tree를 한 단위로 관리하고 kill-on-close로 server crash를 처리한다. 일반 spawn 뒤 assign하면 payload가 그 짧은 구간에 만든 descendant가 job 밖에 남을 수 있다. Microsoft는 suspended flag가 `ResumeThread` 전 실행을 막는다고 정의한다. [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [Process Creation Flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)

**Alternatives considered**:
- `tokio::process::Command` spawn 뒤 Job assign: containment race가 남는다.
- direct child만 terminate: descendant와 server crash 정리가 안 된다.

## R5. 출력 정책 분리

**Decision**: stream마다 다음 중 하나를 선언한다.

1. `ProtocolFrames`: newline-delimited frame 최대 크기, bounded ingress channel. 정확한 frame만 전달하며 초과·malformed·중간 EOF는 typed fatal failure 후 tree 종료.
2. `ParsedCapture`: Git/curl/PATH probe용 bounded complete bytes. overflow는 partial 성공이 아니라 typed failure.
3. `DisplayLog`: terminal/stderr용 byte·event·rate 한도. 초과 부분을 drop/truncate할 수 있고 counters와 marker를 제공.
4. `Null` 또는 명시적 stdin/stdout 상속 금지.

**Rationale**: ACP JSON-RPC frame 일부를 잘라 성공 stream처럼 계속 읽으면 request/response correlation과 permission 의미가 손상된다. 반면 사용자 표시 로그는 가용성을 위해 bounded loss가 허용된다.

**Alternatives considered**:
- 모든 stream에 같은 ring buffer: protocol corruption을 정상처럼 숨긴다.
- 무제한 protocol buffer: newline 없는 peer가 server memory를 고갈시킨다.

## R6. short helper와 blocking 경계

**Decision**: 동일 registry/platform launcher를 쓰는 `run_capture_blocking`과 async facade를 제공한다. blocking facade는 dedicated blocking thread에서만 호출하고, Tokio handler는 `spawn_blocking` 또는 application 전용 blocking port를 거친다. timeout/cancel/shutdown은 registry handle을 통해 동일하게 작동한다.

**Rationale**: `git-core`와 여러 provider port는 현재 동기 API이고 ACP는 async다. 전부 비동기화하면 별도 feature 수준의 파급이 생긴다. 직접 `Command::output()`을 유지하면 shutdown inventory가 분산된다.

**Alternatives considered**:
- Git port 전면 async 전환: 이 증분의 핵심보다 넓다.
- short helper를 supervisor 밖에 둠: user가 명시한 catalog curl/PATH probe와 Git helper 누락을 반복한다.

## R7. 실제 production inventory

**Decision**: 아래를 server-owned로 모두 이관한다.

- `acp-agent-core/infrastructure/acp/runner.rs`: 장기 ACP protocol process.
- `acp-agent-core/infrastructure/acp/terminal.rs`: terminal display process.
- `acp-agent-core/infrastructure/agent_catalog.rs`: `curl` parsed capture.
- `acp-agent-core/infrastructure/acp/util.rs`: login-shell PATH parsed capture.
- `git-core/src/git_cli.rs`: Git history/detail/diff/status parsed capture.
- `workbench-core/infrastructure/git/cli_*_provider.rs`: branch/remote/worktree/change parsed capture.
- `workbench-core/infrastructure/fs/worktree_watcher.rs`: `git rev-parse` parsed capture. `notify` watcher와 debounce thread 자체는 OS child가 아니다.
- `workbench-core/infrastructure/orchestration/worktree_guard.rs`: Git parsed capture.

다음은 별도 범주다.

- `workbench-host/lifecycle/ensure.rs`: desktop/CLI가 daemon을 bootstrap하는 별도 lifecycle owner. 시작된 server 자신의 supervisor에 넣지 않는다.
- AW Tauri `tauri_commands.rs`의 `open`/`rundll32`/`xdg-open`: desktop-native launcher.
- AW `build.rs`: build-time.
- `#[cfg(test)]`, `tests/**`의 cat/git/python/server spawn: fixture.
- GE/MA/HL 등 다른 app production spawn: 045 AW server 범위 밖이며 inventory에 other-app으로 남긴다.

**Rationale**: 2026-09-28 main `20fcd5f`의 production Rust source를 `Command::new`, `process::Command`로 직접 조사했다. helper의 실행 시간이 짧다는 이유로 수명·output·timeout 예외를 두지 않는다.

## R8. inventory drift gate

**Decision**: source scan은 allowlist 방식으로 둔다. `process-supervisor` platform module과 명시된 daemon/native/build/test/other-app 경계 외 production source의 `Command::new`·spawn API가 발견되면 실패한다. 정본은 `contracts/process-inventory.md`이며 위치 변경은 계약과 scan fixture를 함께 갱신해야 한다.

**Rationale**: Rust type system만으로 std/tokio Command의 새 직접 사용을 막을 수 없다. 정적 source gate가 code review 누락을 잡는다.

**Alternatives considered**:
- 문서 표만 유지: 새 callsite drift를 자동 검출하지 못한다.
- workspace 전체 `Command` 금지: 다른 앱, build, test와 daemon bootstrap까지 잘못 막는다.

## R9. server shutdown과 daemon launcher

**Decision**: server drain/force는 owner별 cancel policy를 먼저 적용하고 supervisor `shutdown_all(deadline)`을 마지막 barrier로 기다린다. `ensure.rs` daemon launcher는 기존 startup lock/readiness/process-group 분리 계약을 유지하며 새 server가 준비된 뒤 launcher child waiter만 정리한다.

**Rationale**: daemon launcher를 server 자신의 supervisor에 넣으면 launcher client 종료가 server lifetime을 다시 소유하게 되어 044의 독립 daemon 보장을 깨뜨린다.

**Alternatives considered**:
- 모든 spawn을 문자 그대로 같은 supervisor에 넣음: daemon을 띄우는 주체와 daemon workload owner를 혼동한다.

## R10. 현재 ACP protocol 회귀점

**Decision**: ACP stdout은 `ProtocolFrames`, stderr는 `DisplayLog`로 분리한다. 최대 frame 경계, 1 byte 초과, malformed JSON, 중간 EOF, stderr 무개행 폭주, cancel 동시 발생을 결정적 fixture로 검증한다.

**Rationale**: 기존 runner의 read loop는 protocol stdout과 stderr의 안전 요구가 다르다. bounded drain을 이유로 stdout 일부를 버리면 late response와 permission correlation이 잘못될 수 있다.

**Alternatives considered**:
- 기존 전체-buffer limit만 유지: frame 단위 손상 정책이 명시되지 않는다.

## R11. durable owner와 attempt 저장소 매핑

**Decision**: 기존 SQLite operation ledger schema v2에는 process publication/attempt/outbox table이 없고 `SessionRegistry::reserve_run`, terminal map, watcher/catalog/PATH owner는 memory-only다. platform-neutral foundation은 additive schema v3 `process_publication`과 `process_publication_outbox`만 만든다. T010 뒤 T016의 별도 additive migration이 `process_attempt` containment anchor와 retention을 추가한다. business durability와 containment durability를 구분한다. read-only helper operation/result는 transient지만 production migration 뒤 모든 ServerOwned child의 recovery anchor는 Reaped까지 durable하다.

| Process family | Existing owner boundary | Domain durability | Containment recovery anchor |
|---|---|---|---|
| ACP run | `operation_ledger`의 run start intent + memory `SessionRegistry` | execution/run result + v3 Published/outbox | T016 anchor를 execution/run id에 연결 |
| ACP terminal | run/session 아래 memory terminal map만 존재 | terminal owner + v3 Published/outbox | T016 anchor를 run id + terminal id에 연결 |
| Git/worktree mutation | existing `operation_ledger.execution_id` | 같은 operation retry는 같은 result | T016 anchor를 execution id에 연결 |
| Git read/history/diff/status | authenticated call, durable mutation intent 없음 | transient; crash 뒤 result 재개 없음 | T016 recovery anchor, terminal 뒤 GC |
| worktree watcher Git probe | live watcher subscription/refcount | transient; restart에서 watcher 재구성 | T016 recovery anchor, terminal 뒤 GC |
| orchestration worktree guard | durable orchestration task, 실행은 read-only diff | task id는 durable, helper result는 transient | T016 recovery anchor; task store에 PID를 쓰지 않음 |
| catalog `curl` | cache file은 결과 cache일 뿐 owner가 아님 | server-scoped transient; timeout/failure 시 cache fallback | T016 recovery anchor, terminal 뒤 GC |
| login-shell PATH probe | 기존 durable store 없음 | server-scoped transient; timeout/failure 시 fallback PATH | T016 recovery anchor, terminal 뒤 GC |

**Rationale**: 존재하지 않는 durable terminal/helper business store를 기존 저장소라고 가정할 수 없다. long-lived 또는 side-effecting process만 crash/retry 가능한 domain intent가 필요하다. 그러나 keeper까지 죽는 crash에서 escaped descendant를 식별하려면 read-only helper도 비밀값 없는 recovery anchor가 필요하다. anchor는 result 재생이나 operation retry를 뜻하지 않는다.

**Alternatives considered**:
- helper business result를 영구 기록: catalog/PATH/Git read마다 재생 의미를 만들고 ledger를 팽창시킨다. 대신 containment anchor는 terminal 보존 기간 뒤 GC한다.
- memory SessionRegistry를 durable run owner로 간주: server crash 뒤 판정 근거가 없다.

## R12. accepted response도 adoption 뒤로 이동

**Decision**: 현재 `StartAgentRunUseCase`는 launcher를 background task에서 호출하고 그 전에 run을 반환한다. launcher port를 prepare/adopt와 drive-to-completion으로 나눠 public start가 unpublished lease의 durable publication까지 기다린 뒤 accepted response를 반환하게 한다. 장시간 ACP turn 완료는 계속 background이며 HTTP response와 묶지 않는다.

**Rationale**: Started event 한 줄만 옮겨도 run.start response가 spawn/adopt보다 앞서면 roadmap invariant를 충족하지 못한다. 반대로 전체 agent turn을 기다리면 044의 비동기 run 계약을 깨뜨린다.

## R13. keeper death ownership과 publication outbox

**Decision**: live server는 keeper handle/exit를 감시하고 keeper-only death를 즉시 `containment_lost` arbitration으로 보낸다. adoption 전이면 abort, publication 뒤면 readiness를 내리고 durable anchor로 cleanup ownership을 인계한다. server+keeper death는 다음 startup이 readiness 전에 모든 unfinished anchor를 reconcile한다. Published transaction은 idempotent response와 `(attempt_id,event_kind)` outbox를 함께 commit하며 dispatcher와 reconnect replay는 event id로 dedupe한다.

**Rationale**: next startup만 recovery owner로 두면 live server 동안 descendant가 남고, publication state만 저장하면 commit/send crash gap에서 exactly-once logical event를 복구할 수 없다.

## 미해결 가정과 platform spike gate

다음은 아직 검증되지 않았으며 "미해결 없음"으로 닫지 않는다.

1. descendant가 `env_clear` 또는 새로운 `execve` environment로 attempt nonce를 제거한 뒤 session/group을 이탈할 수 있다.
2. 같은 uid process environment를 읽는 기능이 macOS/Linux의 실제 배포 권한·sandbox·hardened runtime에서 허용되는지 확인되지 않았다.
3. PID/start identity를 확인한 직후 signal하기 전 PID가 재사용되는 TOCTOU를 target handle/pidfd/audit token 없이 막을 수 있는지 확인되지 않았다.
4. live server takeover와 다음 startup을 recovery owner로 설계했지만, keeper hard kill 뒤 T016에서 추가할 durable containment anchor로 escaped descendant를 실제 안전하게 재획득·정리할 수 있는지 입증되지 않았다.

OCR/Codex 설계 리뷰는 nonce 상속 fixture만으로 전체 containment를 주장하지 않는지 검토해야 한다. 구현 task로 넘어가기 전 platform spike는 env 제거+exec, leader 조기 종료, new session/group, double-fork+reparent, control FD close, server hard kill, keeper hard kill, identity-check/signal 사이 PID-reuse 대조를 실제 macOS/Linux에서 실행해야 한다. public API와 권한 안에서 안전한 identity handle을 확보하지 못하면 해당 target은 fail-closed blocker이며 group kill 성공으로 대체하지 않는다.

## R14. 실제 platform 결과와 배포 경계 재설계

**Decision 상태**: T010은 통과하지 않았다. R3의 ordinary keeper+nonce는 빠른 group cleanup과 협력 child handshake에는 쓸 수 있지만 required containment boundary가 될 수 없다. macOS와 Linux 모두 env-clear+session escape descendant가 nonce inventory에서 사라졌고, keeper/server hard kill 뒤 같은 process identity로 생존했다. Windows Job Object 경로만 actual-target feasibility를 통과했다.

**macOS 후보**: 현재 macOS 15 지원 후보는 macOS 11+ 표준 Endpoint Security client를 포함한 signed system extension이다. 전역 fork/exec/exit event를 attempt root부터 필터링하고 audit identity를 tree에 연결하는 방안을 실제 entitlement-bearing release artifact로 spike한다. Apple sample은 Developer ID/provisioning, entitlement, `/Applications` 설치, 사용자 extension 승인과 Full Disk Access를 요구한다. 별도로 descendant-scoped client는 root/TCC 없이 전체 descendant subtree를 관측하지만 availability metadata가 macOS 27.0+라 현재 배포 해법이 아니다. 두 API를 혼합해 현재 지원 근거로 쓰지 않는다. event loss·deadline, extension hard kill, server hard kill, next-start recovery와 exact signal이 모두 성립해야 채택한다. ordinary app keeper만으로 fallback 성공을 반환하지 않는다.

**Linux 후보**: AW server를 systemd service/scope 또는 D-Bus transient unit으로 시작하고 `Delegate=yes`인 cgroup v2 subtree를 attempt별 containment boundary로 쓴다. kernel 문서상 delegatee가 sub-hierarchy를 만들려면 명시적 delegation이 필요하다. payload를 일반 spawn한 뒤 PID를 옮기면 그 사이 실행·fork가 가능하므로 통과할 수 없다. `clone3(CLONE_INTO_CGROUP)` 또는 payload instruction 전에 membership이 완료됨을 결정적으로 입증한 stopped/pre-exec 동등 경로를 사용한다. CI runner의 unified hierarchy와 `cgroup.kill`은 존재했지만 현재 unit 아래 mkdir은 `EACCES(13)`였다. supported distro의 설치된 user/system unit에서 birth-time placement, env-clear/double-fork membership, `cgroup.kill`, `cgroup.events populated=0`, daemon crash와 startup reconcile을 실행해야 한다. direct shell launch에서 delegation을 획득할 수 없으면 명시적 bootstrap 또는 fail-closed UX를 정의한다.

**독립 후속 작업**: publication CAS/outbox storage transaction, pure lifecycle reducer와 output parser/policy는 OS process를 생성·adopt하지 않는 구조로 분리할 수 있다. registry lease/adopt, recovery anchor와 platform launcher는 viable containment adapter 의미에 직접 의존하므로 분리 후보가 아니다. 현재 T010 hard gate는 T011 이후 전체를 막는다. 설계 재리뷰가 gate를 정확한 task 단위로 바꿀 때만 platform-neutral 작업을 시작한다. 상위 AW standalone server 전환 범위와 세 target 지원 조건은 유지한다.

**검증 출처(조회일 2026-09-28)**:

- Apple Endpoint Security: <https://developer.apple.com/documentation/EndpointSecurity>
- Apple macOS 11+ sample: <https://developer.apple.com/documentation/endpointsecurity/monitoring-system-events-with-endpoint-security>
- Apple descendant client Markdown metadata: <https://developer.apple.com/documentation/endpointsecurity/es_new_descendants_client%28_%3A_%3A%29.md>
- Linux cgroup v2 delegation: <https://cdn.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html>
- systemd `Delegate=`: <https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html>
