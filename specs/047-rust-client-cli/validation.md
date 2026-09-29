# 047 구현 검증 기록

## 2026-09-29: T001–T004

환경: Darwin 24.6.0 arm64, macOS 15.6.1. macOS14+/설치본 검증 아님. 설계 기준 bd6099c, 이후 tasks 생성과 client/CLI 공통 기반 작업.

| 명령/검증 | 실제 결과 | 범위 |
|---|---|---|
| `cargo check -p workbench-client -p aw-cli` | exit0 | 신규 crate와 CLI scaffold 빌드 |
| `cargo metadata --no-deps --format-version 1` | exit0 | 두 package workspace membership 확인 |
| `cargo tree -p workbench-client -e normal` 및 aw-cli | exit0 | production graph에 host/core/server/Tauri 없음 |
| 최초 `cargo test -p workbench-client --test limits` | exit101, unresolved limits module | 구현 전 실패 확인 |
| 최초 `cargo test -p workbench-client --test attempt` | exit101, unresolved attempt module | 구현 전 실패 확인 |
| `cargo test -p workbench-client -p aw-cli` | exit0, limits7 + attempt11 = 18개 통과, 실패/무시/filtered0 | pure limits/attempt; CLI 자체 unit test0 |
| `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` | exit0 | 새 client/CLI 모든 현재 target |
| `cargo fmt -p workbench-client -p aw-cli` | exit0 | 현재 신규 Rust 파일 |

Cargo.lock 기존 package version 제거/교체0. 신규 workspace package2와 hmac0.12.1/subtle2.6.1 추가 및 digest subtle edge만 변경했다. 기존 hyper1.10.1/hyper-util0.1.20/http-body-util0.1.3/tokio-tungstenite0.24.0/sha2 0.10.9 유지. hmac0.12는 기존 digest0.10/sha2 0.10 계열과 연결된다.

순수 모델의 request 입력은 불변이며 epoch/instance 교체와 generation 불일치 완료를 거절한다. full reply/revision/replayed와 fault details/outcome을 보존하고 Debug는 private input/reply/fault를 숨긴다. Accepted는 최종 Applied로 판정하지 않는다. 실제 network/store/CLI command wiring은 아직 없다. CLI 현재 entry는 unavailable/exit8 scaffold이며 구현 완료 CLI로 배포할 수 없다.

기존 protocol 및 consumer source 변경0. shared crate의 현재 consumer인 aw-cli를 함께 check/test/clippy했다. 향후 HTTP/WS/identity 연결 시 protocol/host/server/AW 및 TS parity 검증은 T040에 남는다. 원 전체8gate, 실제 merged044 server 통합, 구현 OCR/Codex 리뷰, PR/merge 및 045/046 readiness는 미완료다.


## 2026-09-29: 현재 pane 재개 및 T005/T006/T009/T010

재개 기준 `047-rust-client-cli` HEAD `c4be31895a2ca24dfa961cbab0b0fb4de42471a2`. 최초 tracked 변경0, untracked goal.md와 별도 docs/code-review-app-migration.md만 존재. 두 사용자 파일 stage/commit0. AGENTS/OpenWiki 및 전체047 산출물을 확인했고 extensions.yml 없음, prerequisite script exit0, requirements checklist6/6 완료.

| 실제 명령 | 결과 | 증거 범위 |
|---|---|---|
| `cargo test -p workbench-client -p aw-cli` (재개 직후) | exit0,18개 | T001–T004 현재 소스 재현 |
| `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` (재개 직후) | exit0 | 기존 scaffold strict 검사 |
| `cargo test -p workbench-client --test admission` 최초 | exit101 unresolved admission | 문법 오류 정정 후 구현 전 실패 |
| `cargo test -p workbench-client --test reuse` 최초 | exit101 unresolved ports | 구현 전 실패 |
| `cargo test -p workbench-client --test locator` 최초 | exit101 unresolved locator | 구현 전 실패 |
| `cargo test -p workbench-client --test identity` 최초 | exit101 unresolved identity | 구현 전 실패 |
| `cargo test -p workbench-client --test call_protocol` 최초 | exit101 unresolved http | 구현 전 실패 |
| `cargo test -p workbench-client -p aw-cli` (위 작업 후) | exit0,39개(18+admission5+reuse2+locator6+identity3+call_protocol5) | controlled HTTP 초안 포함. CLI unit0 |
| `cargo test -p workbench-host --lib identify_proof_matches_the_contract_vector` | exit0,1 passed/50filtered | 원 host literal vector와 새 standard HMAC 동일 고정값. 처음 `--exact` 실행은0tests라 통과 근거에서 제외 |
| `cargo fmt -p workbench-client -p aw-cli` | exit0 | 신규 코드 |
| `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` | exit0 | 위39개 및 모든 현재 target |

T005:94 catalog 전체 분류, closed allowlist, unknown 기본 거절, agent identity unavailable/owner fallback0, 실행/서버관리/recover 선행 gate 유지. T006: server/Tauri 없는 독립 fixture consumer의 call/snapshot/reset/close와 private fault debug redaction. T009: read-only FD no-follow 전체 경로, owner/mode/type/link/size 검사 및 literal loopback/schema2 호환 검사. 사용자 지적 반영: descriptor ENOENT만 Unavailable, 권한/symlink는 PrivateState; path diagnostics0 회귀 통과. T010: standard hmac/sha2 verify_slice 및 원 host 고정 vector, 잘못된 nonce/instance/token/hex 거절.

T007의 FD replacement/uid 회귀와 T008의 socket replacement/WS proof는 추가 검증 예정이므로 해당 task 체크하지 않았다. HTTP 초안의5개는 same-socket request 순서, proof 실패 credential0, malformed response/fault/transport Unknown만 검증했다. T011–T015 deadline/cancel/retry 전체 완료 아님. actual merged044 binary/CLI/WS 시험, 구현 리뷰와045/046/전체 roadmap readiness 모두 미완료다.


## 2026-09-29: T007 및 T011–T015 controlled HTTP/cancellation

`cargo test -p workbench-client -p aw-cli` exit0,54 passed/failed0/ignored0: locator 내부2, admission5, attempt11, call_protocol7, call_retry4, http_lifecycle7, identity3, limits7, locator6, reuse2. CLI 자체unit0 유지. 로그 `/private/tmp/aw-047-design/client-progress-tests.log`. `cargo fmt -p workbench-client -p aw-cli` exit0 및 strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0. 이전39개 기록의 최초 합산37 오기는 실제 suite 합39로 정정했다.

T007: FD를 연 뒤 filename 교체에서도 original source만 읽는 회귀, wrong uid metadata와 hardlink 거절 검증. T011: owned hyper HTTP1 sender/driver, identity→same socket handshake/call, bounded headers64/16KiB·body·whole deadline, driver Drop abort/explicit close join. T012: malformed JSON/kind/missing output/empty accepted/status/fault requestId·HTTP/body disagreement 및 body quota 거절, fault 원 details/outcome 및 revision/replayed 보존. 성공 CallReply wire에는 requestId가 없으므로 원 송신 requestId를 owned single-flight 요청에 결합하며 서버가 success requestId를 echo한다고 주장하지 않는다. T013: production admission과 원 typed input DTO를 HTTP/application 둘 다 적용, input·key·protocol invalid는 전송 전 거절.

T014: slow proof/handshake/header/body deadline, 잘못된 handshake epoch, connect future 취소, caller 전체 drop 및 call future만 drop 모두 peer EOF/driver settle를1s안에 확인. proof socket close 후 같은 address에 replacement listener를 실제 bind했고 새로운 연결0/credential0을 확인. retry/recover/cancel 암묵 전송0. T015: controlled peer가 mutation 효과를 key/input map에 기록한 뒤 응답을 유실하고, 새 socket의 fresh proof 후 같은 request/key/input explicit retry에서 effect count1 및 replay 결과를 확인했다. epoch 변경 시 mock transport call0, independent consumer 결과 및 stale completion 거절도 검증했다.

사용자 추가 지적 재현: `cargo test -p workbench-client --test call_retry dropped_execute` guard 적용 전 exit101(Submitted 잔류 assertion 실패). cancellation-safe Submission Drop을 적용한 뒤 call_retry4개 exit0. pending mock+Notify barrier에서 execute future drop→Unknown→같은 identity explicit retry 성공→old generation complete StaleGeneration의 상태 모델 경로를 직접 검증했다. HTTP만 drop하는 시험으로 대체하지 않았다. HTTP Flight Drop도 retained connection을 unusable로 바꾸고 driver를 중단하여 late reply reuse를 금지한다.

T008 HTTP 관련 proof/close/replacement는 검증했지만 별도 WS socket proof는 T028까지 미완료이므로 체크하지 않았다. durable retry/CLI/event recovery/actual merged044 binary 및 subprocess 통합/최종 리뷰·8gate·readiness·PR/merge는 미완료.045/046 및 원 전체 목표 gate 유지.

## 2026-09-29: US2 durable retry 및 finite CLI checkpoint

기준 HEAD `8dde17b` 이후 소스. `cargo fmt -p workbench-client -p aw-cli` exit0, `cargo test -p workbench-client -p aw-cli` exit0 **85 passed/failed0/ignored0/filtered0**, strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0. 실제 로그 `/private/tmp/aw-047-design/us2-checkpoint-tests.log`. client 기존54+retry_store12=66; CLI unit2+cancellation4+finite8+retry_process5=19. retry_store12에는 환경변수 없이 no-op인 child helper1이 포함되며 parent 시험이 별도 test subprocess lease 및 abrupt exit17을 실행했다. finite8의 fault family 시험1개 안에서14종의 실제 aw subprocess를 순차 실행했다. lock package version 변경0; aw-cli hmac/libc(test fixture), sha2(안전한 key filename namespace) dependency edge3 추가.

- `cargo test -p workbench-client --test retry_store`: mutex 적용 전 Arc 공유 begin/publish 경쟁에서 winner2 재현(exit101); 적용 후 전체 read-check-write transaction 직렬화로 publish/complete/begin_retry 각각 winner1, loser StaleGeneration 및 첫 결과 유지. 별도 process flock 시험으로 이를 대체하지 않았다. lock FD inode 확인, no-follow private state, bounded body/input, fsync file→rename→fsync directory publish/CAS 구현. 실제 process abrupt exit17 후 정확한 Unknown/key/input/instance/epoch 복원. fsync/rename 실패 주입은 T017에 남아 있다.
- retryable Unknown/NotApplied fault reopen 시험은 수정 전 StaleGeneration으로 exit101; Attempt와 같은 shared policy 적용 후 통과. Applied/nonretryable/Accepted/Complete는 완료 cache이며 자동 resubmit0. restored input budget 시험은 수정 전 exit101; load 시 Input 한도 재적용 후 통과.
- 최초 finite CLI scaffold 시험은 exit101(3failed/2passed). 현재 catalog94, strict argv, invalid UTF8/JSON/schema/+1 quota, nonUTF8 argv, full fault14종의 code/outcome/requestId 및 safe diagnostics, generic/explicit run.start/run.cancel prerequisite parity가 actual subprocess로 통과했다. RunCancel은 계속 gate 거절이며 typed RunCancelInput 및 bench lookup 투영을 준비했다. 미래 실제 cancel 동작/exit6 검증이나 gate 활성화 proof로 주장하지 않는다.
- actual retry_process5: response loss 뒤 같은 key/request/input explicit replay와 effect1, 완료 cache HTTP0, key/input 변경 전송0, epoch 변경 command0, pre-send private store 실패 mutation0(identify/handshake2회만), missing descriptor/Unavailable/Identity preflight 뒤 원 Unknown/requestId/receipt 보존. 실제 첫 CLI가 제출한 상태에서 두 번째 invocation은 active lease로 거절하고 HTTP0; 첫 CLI SIGKILL 및 reap 후 정확한 Unknown 복원, 같은 wire request 재시도 effect1을 확인했다.
- `cargo test -p aw-cli --test cancellation` 최초 unfinished-stdin 시험 exit101(1failed/2passed). yield_now와 finish의 stdin EOF 때문에 proof가 없던 구성은 제거했다. SIGINT handler를 application spawn 전에 동기 등록하고, 별도 pipe read FD의 FIONREAD로 child의 입력1byte 소비를 확인한 뒤 SIGINT를 보낸다. writer는 child wait/reap까지 열려 있으므로 EOF 종료가 아니다. 이 시험과 제출 후 SIGINT130/Unknown receipt/implicit cancel0, local deadline7/Unknown/implicit cancel0 통과.
- broken stdout pipe 시험은 최초 exit0을 재현해 exit101. Tokio 출력 write 이후 flush 완료를 확인하고 안전한 stderr outputUnavailable/exit8을 적용한 뒤 통과. stdout 유실을 mutation NotApplied로 바꾸지 않으며 receipt의 기존 outcome을 유지한다. fixture 모든 입출력/wait는3s bounded, timeout은 kill+1s reap 경로를 가진다. panic 경계의 직접 시험은 T022에 남았다.

T014 replacement proof 정정: 이전 close 이후 bind 시험은 identify 직후 새 TCP를 여는 회귀의 경합 범위를 입증하지 못했다. 현재 `proof_socket_replacement_is_listening_before_identify_response_release`는 old listener를 닫고 accepted socket을 유지한 상태에서 replacement 실제 bind/listen ready barrier를 확인하고 valid identify+Connection:close 응답을 release한다. 실제 replacement peer connection0/credential0을 확인했다. controlled fixture이며 actual merged044 binary 시험이 아니다.

T018–T021/T023 체크, T016(exit6 pending), T017(fsync/rename failure), T022(panic boundary)는 미완료로 유지한다. WS/recovery/JSONL, exact20fcd5f actual server, 구현 OCR→Codex review, root8gates,045/046·TUI/MCP·signing/update·thin desktop 전체 readiness와 PR/merge 모두 미완료다. goal.md는 최신 진행/재개 안내만 갱신하는 untracked 사용자 파일이며 commit 제외. 별도 docs/code-review-app-migration.md 수정/stage/commit0.

### US2 저장 실패 및 panic 경계 추가 검증

`cargo test -p workbench-client --lib sync_and_rename_failure` exit0/1passed(2filtered): 빈 store의 최초 publish와 기존 state의 generation2 CAS 모두 FileSync/Rename/DirectorySync 직전 오류를 주입했다. FileSync/Rename 최초 실패는 state 없음; DirectorySync 최초 실패는 rename된 원 request/key/input/instance/epoch/generation1/Unknown/resultNone을 유지하지만 PrivateState 오류를 반환했다. CAS 실패는 rename 전 generation1, rename 뒤 generation2 Unknown이며 temp residue0. **syscall 경계 오류 주입**이며 실제 syscall 실패/재부팅 durability proof가 아니다. 각 injected 오류의 실제 aw subprocess command0 시험은 아직 별도로 연결하지 않았으므로 T017 체크를 보류한다. 앞선 실제 CLI의 private store open/publish 실패 mutation0 증거와 혼동하지 않는다.

`cargo test -p aw-cli --bin aw dependency_panic` exit0/1passed(2filtered): spawned task의 실제 panic JoinError를1s bounded join하고 production job_result 경계에서 internal1/Unknown safe envelope로 변환하며 raw panic sentinel0을 확인했다. production entry의 panic hook은 payload 로그를 억제한다. 이 직접 경계 시험은 실제 aw 바이너리에 panic을 주입한 subprocess proof가 아니다. 기존 actual subprocess의 SIGINT130/unfinished input/deadline/broken stdout/invalid input 및 stderr/stdout golden과 함께 T022 검증을 구성한다.

`cargo test -p workbench-client -p aw-cli` exit0/87passed/failed0/ignored0/filtered0; client67/CLI20. 로그 `/private/tmp/aw-047-design/us2-failure-checkpoint-tests.log`. T016 exit6 및 T017 injected-error actual command0는 미완료 유지.

### T017/T022 보완 및 T024/T026 reducer checkpoint

최종 `cargo fmt -p workbench-client -p aw-cli` exit0; `cargo test -p workbench-client -p aw-cli` exit0 **104passed/failed0/ignored0/filtered0**(client82/CLI22); strict clippy 동일 명령 exit0. 로그 `/private/tmp/aw-047-design/reducer-owner-final-checkpoint-tests.log`. client unit5(기존 locator2 + syscall 경계1 + ownedHTTP 실패1 + FD lease1), 기존 integration52 + retry_store12 + event_reducer13 =82. CLI unit5 + finite8 + cancellation4 + retry_process5 =22. libtest helper no-op1씩은 retry_store12/CLI unit5에 포함되며 parent 시험에서 실제 subprocess를 실행했다.

- `cargo test -p workbench-client --lib initial_sync_rename_failures` exit0/1passed(3filtered 당시). 최초 publish의 FileSync/Rename/DirectorySync 경계 오류가 CLI와 동일 `publish_attempt` use case를 통과해 owned HTTP와 연결됐다. 모든 경우 PrivateState 반환, actual peer identify+handshake2/command0/effect0. rename 전 state 없음, rename 뒤 원 exact identity Unknown state 존재/성공 반환0을 구분했다. injection adapter는 cfg(test)만 존재하고 production은 실제 sync/rename syscall을 실행한다. 실제 aw 바이너리에 오류 trigger를 넣은 시험이나 재부팅 durability proof가 아니다. 앞선 CAS failure 시험과 actual CLI persistence 실패 mutation0도 유지. 이 보완 후 T017 체크.
- `cargo test -p aw-cli --bin aw production_hook_in_isolated` exit0/1passed(4filtered). 동일 production `install_panic_hook` 함수를 실행하는 격리 **test executable** 안에서 실제 panic→JoinError→safe finite projection→exit1을 실행했다. stdout/stderr raw sentinel0, stderr JSON1/newline1, exit1,3s bounded wait와 timeout kill+1s reap를 검증. stdout에는 libtest banner만 있고 application JSON/log0이며 production panic trigger flag/env0. test helper 환경변수는 cfg(test) 코드에서만 읽는다. 이전 JoinError unit만으로 hook을 검증했다고 기록한 범위는 이 시험 전에는 충족하지 못했다. 이 보완 후 T022 체크.
- event_reducer 최초 실행 exit101 unresolved events module. 이후13개 passed. 사용자 지적의 ConsumerId 교차 및 earlier-join live2→replay1 시험은 수정 전 exit101(11passed/2failed)로 직접 재현했다. owner 고유 ID를 ConsumerId에, Arc owner와 consumer/stream generation을 Delivery/Reset에 묶었다. 다른 reducer handle은 queue/applied/inflight/reset/unregister 변경0. retained queue는 순서 정렬·중복 제거·연속 sequence ACK를 적용해 seq1 없이 seq2를 ACK하지 않는다. 같은 revision의 서로 다른 sequence는 독립 소비한다. notification stream은 reconnect snapshot 정책이며 retained stream은 replay. T025 async snapshot/replay/live 교차와 WS transport 검증은 아직 없다.
- 전체 검증 중 retry_store immediate reopen2가 PrivateState로 실패(exit101/그 suite10passed2failed)했다. 별도12개 재실행은 통과했으므로 이를 통과로 덮지 않았다. shared lock FD를 복제해 owner store Drop 뒤 reopen 거절을 결정적으로 재현한 unit은 exit101. Drop에서 explicit LOCK_UN을 적용한 뒤 통과했고, duplicate FD 종료가 새 lease를 해제하지 않음도 확인했다. concurrent process launch의 일시 FD 공유가 원 간헐 실패 원인이었을 가능성은 추론이며 원 실패의 errno 직접 증거는 없다. 최종 전체104개와 strict clippy는 수정 후 통과했다.

현재 T016의 production cancel-rejected6, T008 WS proof, T025/T027 이후 recovery/WS/JSONL/actual merged044/전체 roadmap readiness/최종 리뷰·PR·merge는 미완료다. 045/046 gate를 활성화하거나 완료로 주장하지 않는다.

### T025 live-first recovery 및 terminal boundary checkpoint

기준 HEAD `dea75b9` 이후 소스. `cargo fmt -p workbench-client -p aw-cli` exit0, `cargo test -p workbench-client --test recovery` exit0 **22passed/failed0/ignored0/filtered0**. `cargo test -p workbench-client -p aw-cli` exit0 **126passed/failed0/ignored0/filtered0**(client104/CLI22). 로그 `/private/tmp/aw-047-design/recovery-checkpoint-tests.log`. strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0. 원 `packages/workbench-client/src/event-client.races.test.ts` 및 gap/reconnect/listener fixtures와 대조했다. controlled model/async port 시험이며 실제 WS·merged044 binary·aw JSONL proof가 아니다.

- live-first Connect(boundary)→hello→snapshot(actual applied/live boundary 별도)→각 consumer reset 완료→snapshot 포함 sequence 필터링→연속 event ACK. hello만으로 budget을 초기화하지 않으며 snapshot 실패5회는 Exhausted. reconnect 중 동일 pending snapshot/reset의 boundary 유지, live7 먼저/replay6 나중이어도 ACK6→7, Live에 earlier listener 합류 시 새 ticket cursor0 필요→live2 먼저/replay1 나중 ACK1→2를 검증했다. pending load/reset 중 합류 listener는 자체 actual applied context와 full snapshot coverage를 쓰며 제거된 reset은 stale다. SnapshotPort는 delta가 아닌 full state/journal coverage를 반환하는 계약이다.
- 모든6종 gap reason의 새 epoch ×옛 load 성공/실패/reset 완료3종에서 stale·cursor0·새 epoch 복구를 검증했다. 초기 red exit101은 UnknownStream에서 새 epoch의 lastSequence를 재사용했고, reason shortcut 이전 공통 rebind/round 무효화 후 통과했다. 옛 snapshot_failed도 새 round를 변경하지 않는다.
- 사용자 지적 stream exhaustion은 recovery_attempts1, stream reset 실패→listener Load→새 gap Exhausted 경로를 직접 재현했다(red exit101). listeners.clear와 reducer pending stream/consumer generation·queue 무효화 및 terminal completion guard를 적용했다. 옛 listener load 성공/실패·reset·delivery ACK4종 모두 StaleGeneration이며 phase와 applied/min cursor 변경0. listener-only exhaustion은 Live를 유지하고 다른 consumer ACK1/2를 허용하며 느린 consumer 제거 후 min cursor2다.
- async pending source Notify barrier→new gap→explicit cancel+bounded destructor settle→새 snapshot 완료, callback pending Drop/explicit cancel ACK0, slow reset 동안 다른 consumer의 event6 처리, old slow reset stale를 검증했다. snapshot timeout은 원 boundary 재시도이며 ACK0. callback panic은 수정 전 join Err(Protocol)로 토큰 유실(red exit101); future unwind를 safe completion으로 변환한 뒤 원 delivery identity를 보존해 listener resync로 진행한다. library global panic hook 억제 proof로 주장하지 않는다.
- jitter 최소값이250ms 아래로 내려간 시험은 exit101(1failed/20filtered 당시). min/max clamp 후 전체22개 통과. notification stream disconnect는 live hello 뒤 snapshot/reset, retained stream disconnect는 applied cursor replay로 구분했다.

T025 체크. T027은 상태 모델 및 owned callback/snapshot helper만 구현된 상태이며 **실제 managed session의 socket/task 취소·bounded join 연결은 미완료**이므로 체크하지 않는다. T008 WS proof/T016 production cancel6/T028 이후 실제 WS/JSONL/server/subprocess 및 구현 리뷰·root8gate·전체 roadmap readiness·PR/merge는 남아 있다.045/046 등 production gate 활성화0. `goal.md` 진행 안내만 갱신하며 commit 제외; 별도 `docs/code-review-app-migration.md` 수정/stage/commit0.

### T008 별도 WS proof 및 owned transport checkpoint

기준 recovery checkpoint `083b6ec` 이후 소스. `cargo fmt -p workbench-client -p aw-cli` exit0; `cargo test -p workbench-client --test ws_protocol` exit0 **14passed/failed0/ignored0/filtered0**; `cargo test -p workbench-client -p aw-cli` exit0 **140passed/failed0/ignored0/filtered0**(client118/CLI22); strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0. 전체 로그 `/private/tmp/aw-047-design/ws-proof-final-checkpoint-tests.log`. 기존 dependency version 변경0; shared controlled HTTP/WS fixture를 실제 CLI subprocess 시험에서도 쓰기 위해 aw-cli dev-only futures-util/tokio-tungstenite dependency edge2 추가. production host/core/server/Tauri dependency0 유지.

- 실제 controlled TCP에서 HTTP identify→handshake→ticket POST 후 그 sender/driver를 retire하고, 별도 WS TCP fresh nonce identify proof→동일 socket GET upgrade→verified hello 순서를 검증했다. Authorization은 HTTP handshake/ticket만 존재하며 두 identify/WS upgrade는0. 두 nonce는 서로 다르며 ticket cursor는 반환받은 완전한 orchestration binding stream 주소를 사용한다. wrong proof on second socket은 ticket upgrade0. HTTP upgrade는 표준 Sec-WebSocket-Accept를 standard tungstenite helper로 확인한 뒤 hyper Upgraded 소유권을 넘기고 driver를 bounded join한다. proxy/pool/자동 reconnect/HTTP retry0.
- WS replacement fixture는 첫 socket에서 identify/handshake/ticket을 완료한 뒤 둘째 accepted proof socket을 유지한 상태에서 old listener를 닫고 replacement를 실제 bind/listen했다. ready barrier 이전 client 완료0 확인→valid identify+Connection:close release→client 실패→replacement queued accept 우선 drain 결과 연결0/Authorization0/ticket0. 처음 connect가 이미 끝난 뒤 replacement를 bind한 시험이 아니다. 이 보완 후 T008 체크.
- hello 이전 gap, wrong hello epoch, binary/malformed/unknown frame/schema/stream, frame header의 선언 길이 +1와 fragmented message 총량 +1을 typed error로 거절하고 실제 peer EOF를 bounded settle했다. 전체 read future만 drop해 retained connection 객체가 살아 있어도 다음 read Unavailable/peer EOF이며 implicit new connection0. 별도 owned read task는 request timeout100ms보다 긴200ms idle에도 종료하지 않았고 abort+bounded join/peer EOF를 검증했다.
- 표준 tungstenite가 RFC6455 decoding/UTF8/masking/fragment validation을 수행한다. 얇은 FrameIo는 frame boundary별 AsyncRead cap과 첫 byte부터의 partial-message deadline만 관리한다. completed hello 뒤 같은 raw write에 포함된 incomplete header 및 unfinished fragment는 Deadline; fragment 중 ping도 deadline을 초기화하지 않는다. idle에는 timer 없음. frame/message는 각각1MiB 기본이며 queue256/8MiB는 기존 reducer 한도다. 실제 managed reader의 aggregate queue 연결/pressure proof는 T027/T029에 남는다.
- ticket issue/expired upgrade HTTP401은 full fault/status 및 서버가 생성한 trace ID를 보존하고 refresh/retry0. expiresAt은 응답 형태만 검사하고 실제 expiry enforcement는 서버 책임이다. 시험은 controlled401이며 실제 서버 TTL30s/재사용 proof가 아니다. ticket query percent encoding과 safe Debug를 검증했고 URL/ticket/owner credential을 diagnostics에 출력하지 않는다. candidate production stream allowlist는 현재 Orchestration/Bench만이며 Run/Worktree/Exchange는 prerequisiteUnavailable, invalid epoch/duplicate cursor는 HTTP0. 045/046 readiness bool/env bypass0.

처음 WS test module 미등록은 exit101. shared fixture 추가 뒤 strict clippy는 CLI test dependency 누락 및 formatting lint로 exit101; dev dependency edge와 formatting 수정 뒤 최종 strict exit0. ticket fault 시험은 처음13passed/1failed(exit101): fixture에 HTTP problem `status` 필드가 빠져 adapter가 Protocol로 올바르게 거절했다. 원 server problem 형태로 fixture를 수정한 후14passed 및 전체140passed다. 최초 failed 전체 log를 PASS로 주장하지 않는다.

T027 actual session callback/snapshot/socket ownership 연결, T028 aggregate queue 및 T029 socket recovery/pressure를 포함한 나머지 task는 체크하지 않았다. CLI JSONL/actual exact20fcd5f binary/subprocess/review/root8gate/045·046 및 전체 목표 readiness·PR/merge 미완료를 유지한다. 별도 사용자 docs 파일과 untracked goal은 commit 제외.

### T027–T029 managed session, aggregate budget 및 reconnect checkpoint

기준 HEAD `5b9839f` 이후 소스. `cargo fmt -p workbench-client -p aw-cli` exit0, `cargo test -p workbench-client --test session`에서 notification 추가 전14passed/failed0/ignored0/filtered0. `cargo test -p workbench-client --test session notification_disconnect` exit0/1passed/14filtered. 최종 `cargo test -p workbench-client -p aw-cli` exit0 **155passed/failed0/ignored0/filtered0**(client133/CLI22; session15와 기존 ws_protocol14 포함). 로그 `/private/tmp/aw-047-design/session-final-checkpoint-tests.log`. strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0 및 `git diff --check` exit0. dependency 변경0.

- 실제 controlled TCP/WS peer의 immediate gap은 opened port 진입 Notify 뒤 release했다. 기존 cancel_callbacks가 opened를 취소한 경로는 exit101(open Drop1)로 재현했다. opened 전용 owned JoinSet과 pending reset gate로 성공한 opened 이전 reset/consume0, 성공 후 reset5/ACK6, pending open 도중 stop의 reader EOF/open abort+bounded join/cursor0을 검증했다. readiness는 callback 진입·physical peer gate이며 단일 yield_now나 stdin EOF를 근거로 쓰지 않는다.
- reader frame의 queue lease를 channel send 이전 획득하고 channel frame·reader pending-send·reducer queue/inflight를 하나의 Mutex budget으로 계산한다. actor는 lease를 reducer queue로 원자적으로 이전하고 ACK/reset/gap도 같은 ledger를 갱신한다. slow consumer callback 진입 뒤 burst에서 item3 및 serialized envelope bytes3개 한도 exact, +1 Limit/ACK0/cleanup usage0/peak quota이하를 검증했다. frame1+channel1을 임시 frame1이라고 주장하지 않는다. bounded RFC frame buffer 자체는 별도 frame/message 한도로 제한되며 transport가 decoder에서 거절한 +1 frame은 성공적인 pending item으로 세지 않는다.
- actual socket gap5→pending snapshot port→새 socket gap6에서 옛 snapshot Drop barrier 및 새 load/reset6/ACK7, ticket cursor[0,5,6]을 확인했다. stream terminal eviction은 reader retire 후 snapshot/reset5만 적용하고 reconnect0이다. listener-only exhaustion은 실패 consumer의 generation만 소진하고 다른 consumer가 실제 wire event1/2를 소비하며 min cursor0을 유지한다. whole session future Drop은 pending snapshot 또는 consume callback과 owned reader를 모두 abort하며 실제 destructor/EOF/Terminal/cursor0/queue0을1s bounded 확인한다. explicit stop/error는 abort+bounded join cleanup_error도 별도로 확인한다. library global panic hook을 억제하는 시험은 아니다.
- 실제 socket disconnect 후 descriptor는 유지하고 replacement listener를 닫아 첫 reconnect에 실제 TCP Unavailable을 만들었다. 원 구현은 첫 실패에 세션 종료하여 target event2 timeout(exit101)이었다. transient Unavailable/Deadline/TransportUnknown 및 계약의 retryable transient fault만 같은 applied 또는 pending boundary cursor로 bounded attempt/jitter backoff 재시도한다. successful reconnect 후 [0,1,1] cursor, ticket[0,1], command0, cleanup0. gap 중에는 [0,5,5] boundary와 snapshot actual applied0을 구분했다. exhaustion은 initial+5 exact 연결이며 cursor1을 유지한다. wrong proof/Protocol/401 auth는 실제 peer 각각 request1/3/3 및 initial+1 exact connect에서 terminal이며 retry0.
- full snapshot/reset 성공도 reconnect budget을0으로 만든다. 이벤트 없이 gap→connect→hello→snapshot→모든 reset Live를6회 반복해 snapshot6/ticket7/cursor6 및 각 회차 Live barrier를 확인했다. hello만으로 초기화하지 않는다. slow reset이 pending인 동안 fast consumer ACK6/7/8이 매번 성공하고 actual socket close가 이어지는 회귀는 이전 unconditional Delivery 성공 reset에서 연결7(expected4)로 exit101이었다. phase Live에서만 delivery 성공으로 초기화하고 stream round의 마지막 성공 reset으로도 초기화한 뒤 initial+3 exact connects/[0,5,5,5]/applied0/slow reset 취소로 통과했다. 일부 reset 미완료·consumer 실패는 성공 recovery로 계산하지 않는다.
- notification Bench stream은 실제 event1 ACK 후 disconnect시 ticket cursor1의 새 socket/verified hello를 먼저 확보했다. snapshot port를 explicit barrier로 pending 유지하는 동안 buffered notification2/3 소비0/applied1; snapshot2 release→reset2→event3 ACK로 event[1,3]/cursor3, ticket[0,1], queue0/peer EOF/cleanup0을 확인했다. retained stream replay 시험으로 이를 대체하지 않았다.

초기 session compile E0597 및 strict clippy의 test MutexGuard await/needless range loop 오류는 exit101이었고 scoped guard/enumerate 수정 후 최종 검증을 실행했다. 위 red 회귀의 실패를 최종 PASS로 덮지 않는다. controlled source의 SnapshotPort는 fixture state를 제공한다. 실제 HTTP snapshot source·CLI JSONL·exact20fcd5f binary/subprocess(T030–T038)는 아직 미완료이며 이 checkpoint를 actual adapter conformance나 US3/047/전체 전환 완료로 계산하지 않는다.045/046 활성화0, RunCancel gate와 exit6 pending 유지. 별도 사용자 docs2개 및 untracked goal은 commit 제외한다.

### T030 JSONL consumer 준비 및 종료 직전 성공 ACK checkpoint

`cargo test -p workbench-client --test session stop_after_successful_callback`는 수정 전 exit101(applied0, expected1)로 실패했다. callback이 완전하게 성공했지만 actor가 completion을 수거하기 전에 stop이 ready이면 model.close가 generation을 먼저 무효화했다. cleanup에서 reader와 callback/open 작업을 먼저 abort+bounded join하고, 같은 generation의 이미 성공한 delivery/reset completion만 ACK한 뒤 model을 close한다. pending/aborted/failed callback과 stale generation은 ACK하지 않는다. 최종 SessionResult cursor는 cleanup 뒤 값을 사용한다. 수정 후 같은 명령 exit0/1passed/15filtered.

공통 `JsonlOutput`/EventConsumer는 newline 포함 write_all과 flush 완료만 성공으로 반환한다. write 도중 오류나 enclosing future Drop은 sink를 retire하여 부분 record 뒤 stream.end/다음 event를 붙이지 않는다. 성공/오류 stream.end는 safe CliError 필드만 투영한다. stdout FD나 actual command lifetime composition은 아직 연결하지 않았다. 테스트가 같은 production consumer를 import하도록 CLI use case/inbound/infrastructure를 library target으로 분리했으며 executable에는 signal/runtime/exit/panic hook이 남는다. 기존 finite subprocess/production panic hook 시험은 그대로 통과했다. async-trait production dependency edge1 추가, 기존 lock package version 변경0.

`cargo test -p aw-cli --test stream_output` exit0 **6passed/failed0/ignored0/filtered0**: one-byte partial write의 open/event/reset/end JSON4개, event 중 broken pipe 이후 newline0/end append0, 부분 event future Drop, newline이 있어도 flush pending이면 open 성공0, 실제 controlled WS session의 완전한 event flush 직후 stop→ACK1/end cursor1, actual session의 partial write readiness→stop→callback abort/ACK0/end append0/peer EOF/queue0을 검증했다. 두 actual socket 시험은 writer poll/flush의 Notify barrier를 사용한다. 실제 aw SIGINT/OS stdout pipe proof로 대체하지 않으며 아직 T031 전체 체크하지 않는다.

`cargo fmt -p workbench-client -p aw-cli` exit0; 최종 `cargo test -p workbench-client -p aw-cli` exit0 **162passed/failed0/ignored0/filtered0**(client134/CLI28). 로그 `/private/tmp/aw-047-design/jsonl-consumer-checkpoint-tests.log`. strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0, `git diff --check` exit0. 최초 consumer module 미구현/path 해석/Limits API 이름 오류는 각각 compile exit101이며 정정 후 전체를 재검증했다. direct fixture의 fault projection unit을 중복 실행한 최초5개 집계 대신 library import 후 고유6개 결과를 사용했다.

T030/T031은 **미완료**다. 다음은 production HTTP snapshot source(반환 bindingId/benchId/workspaceId 구별), actual `aw events watch` 및 run watch의 동일 admission/JSONL 연결, 취소 가능한 OS stdout write와 SIGINT/final stream.end/finite preflight 경계다. T032/T033 actual ordering/subprocess와 T034–T044 actual server/review/전체 readiness/PR·merge 모두 남는다. user docs2개와 goal stage/commit0,045/046 readiness 활성화0 유지.
