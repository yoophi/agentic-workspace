# 047 구현 검증 기록

## 현재 적용할 완료 범위 (2026-09-29 최신 사용자 지시)

047 Rust client/CLI 자체의 가능한 모든 계약·actual exact20fcd5f 서버 및 aw subprocess 시험·affected/root8 검증·OCR delegate → Codex adversarial --wait 순차 리뷰/수정·PR/CI·squash merge·main checkout/pull·`docs/047-completion-handoff.md` 기록 후 중지한다. 현재 이 조건은 미충족이며 작업 중이다. 045/046·TUI/MCP·서명/update·desktop fallback 제거는 후속 미완료로 인계하고 production gate를 유지한다. 후속 구현을 시작하지 않으며 전체 전환 완료로 주장하지 않는다.

T016 cancel-rejected exit6은 닫힌 RunCancel prerequisite 때문에 production 미검증/이연이다. 현재 도달 가능한 exit 검증과 explicit/generic gate parity를 유지하며 이연을 최종 리뷰받는다. 후속 실제 proof 없이 활성화하지 않는다. T043은 전체 roadmap 구현 대신 위 미완료 사실/gate/재개 조건의 인계 검증으로 변경됐다. 아래 기존 날짜별 checkpoint는 당시 실제 실행 결과와 범위의 이력으로 보존하며, 과거 전체 readiness 요구가 최신 047 종료 기준을 대체하지 않는다.

현재 checkpoint HEAD `be9852c91ef7ddc2609c855cac77fa4030d1b422`, branch `047-rust-client-cli`. T001–T039의 구현 및 controlled/actual wire checkpoint를 완료했고, 최종 workspace 검증과 구현 리뷰 수정은 작업 중이다. PR/CI/merge/main sync/인계는 아직 완료하지 않았다. goal.md는 untracked 유지, 별도 사용자 docs2개 수정/stage/commit 금지.

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

## 2026-09-29: T030–T033 실제 CLI streaming 및 controlled ordering

기준 HEAD `05e55a395be27a3c8b6d66e9fe557980e7ad6d5f` 이후 변경. 최신 사용자 종료 기준 변경(spec/plan/tasks/contracts/goal 및 이 문서)도 최종 구현 리뷰에 포함한다.

- `cargo fmt -p workbench-client -p aw-cli` exit0. `cargo test -p workbench-client -p aw-cli` exit0 **187passed/failed0/ignored0/filtered0**(client136/CLI51), `/private/tmp/aw-047-design/cli-stream-ordering-tests.log`. `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0, `/private/tmp/aw-047-design/cli-stream-ordering-clippy.log`. `git diff --check` exit0. Cargo.lock은 aw-cli test-only bytes/http-body-util/hyper/hyper-util/uuid dependency edge5만 추가했으며 package version 변경0. production graph의 host/core/server/Tauri0 유지.
- `cargo test -p aw-cli --test event_snapshot` exit0 **4passed**: binding/bench/workspace 식별자를 구별하는 실제 HTTP readonly bench.list→orchestration.get, absent/duplicate/malformed snapshot 및 epoch/gated 요청0. live hello 이후 fresh source를 쓰며 applied/boundary와 revision99/sequence5를 구별한다. notification snapshot은 notificationsRetained:false이며 저장되지 않은 title/state를 만들지 않는다.
- stdout unit **5passed**(위 전체 library suite 포함): 정상 Drop의 원 blocking/nonblocking flags, full pipe pending writer 취소+join, FD consume 뒤 AsyncFd registration 실패의 원 flags 복구, /dev/null 무등록 쓰기, /dev/zero 등 unsupported char 거절. F_DUPFD_CLOEXEC가 공유하는 OFD의 flags를 stable owned restoration FD lease로 복구한다. regular file의64KiB cap은 syscall당 byte 한도이며 syscall 시간 상한 증거가 아니다.
- `cargo test -p aw-cli --test stream_output` exit0 **8passed**: partial/newline/flush 경계와 actual session ACK1/partial ACK0, 늦은 old-generation 성공 delivery/reset을 새 gap 뒤 완료해도 StaleGeneration 및 cursor0. 마지막2개는 production JsonlConsumer와 EventRecovery/OwnedJob 조합의 flush readiness/waker barrier이며 managed socket 취소 proof를 대신하지 않는다.
- `cargo test -p workbench-client --test event_ordering` exit0 **2passed**: private test-only HarnessConnection의 fresh nonce proof→same TCP handshake/recover request를 실제 peer가 받은 `/v1/calls` + operation barrier를 확인한 뒤 reply/event gate를 release한다. bootstrap s7/r40→runtimeReconciled8/r41→notificationRecovery9/r41 exact vector, ACK7→9, HTTP reply 및 final snapshot41을 양방향 비교한다. reply-first에는 event gate가 닫힌 동안 record1/cursor7, events-first에는 request 수신 이후 두 ACK 완료 때 reply gate가 닫혀 call 미완료임을 확인한다. outer timeout/panic 뒤 owned call/session abort+bounded join; Drop도 abort하며 detach하지 않는다. production recover admission은 변경0, raw test authority는 production library에 포함되지 않는다.
- `cargo test -p aw-cli --test stream_process`는 최종 전체 실행에서 **12passed/failed0**: actual open→sameRev 두 event→SIGINT end/130·EOF; idle request timeout보다 오래 생존; invalid frame 뒤 safe end/1; /dev/null redirect 실제 subprocess/130/EOF; shared stdout socket OFD 원 nonblocking false/true × SIGINT130/protocol1 flags exact 복구와 부모 writer 재사용; live-first gap→HTTP snapshot→reset→event6; invalid/gated preflight의 stdout0/JSON1/HTTP0; initial proof/hello fault; pending proof SIGINT; broken pipe의 resnapshot/implicit command0; 실제700KiB event의 socket backpressure POLLOUT/FIONREAD readiness 후 SIGINT130/reap·원flags복구/partial newline0/end append0/EOF; bootstrap→recover request 수신 barrier→두 order JSONL exact vector·독립 EventConsumer/ACK reducer parity·snapshot/end cursor3/reap. full stdout pipe에서는 stream.end 성공을 주장하지 않으며 library partial proof가 ACK0을 별도로 확인한다.

실패와 수정: 최초 전체182 시도는 pending-proof SIGINT fixture가 halt_at="identify"로 실제 경로와 불일치하여 request2/expected1(exit101/해당 suite10passed1failed)이었다. `/v1/system/identify`로 수정 뒤182 전체/strict Clippy exit0. Clippy question_mark lint2는 exit101; 기계적으로 ? 적용한 partial move compile exit101 뒤 terminal && result.is_err() 반환으로 수정했다. T032 최초 production recover call은 Admission으로 거절(exit101/2failed)되어 private test-only owner harness로 분리했으며 production gate를 해제하지 않았다. 이후 helper method/path compile exit101과 fixture command 자동 idempotency key의 effect-map count1/expected0 실패를 수정하여 recover identity1/retry0을 exact 비교했다. T033 helper 상대 경로 오류 compile exit101은 올바른 경로로 정정했다. 최종187 결과로 위 실패를 덮어 PASS라고 주장하지 않는다.

macOS shared flags 시험의 최초 raw whole flags 비교는 syscall write 뒤 Darwin FWASWRITTEN(0x10000) kernel bit가 추가되어 실패했다. 원 flags를 잡기 전 같은 libc::write로 그 bit를 미리 설정한 뒤 F_GETFL 전체 값을 비교하며 O_NONBLOCK을 마스킹해 숨기지 않는다. [Apple XNU fcntl.h](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/fcntl.h)의 FWASWRITTEN/FCNTLFLAGS 정의와 관측을 구별한다. SIGINT shared-output fixture의 gate 미release→peer.settled timeout은 gate 없는 hello peer로 바꾸어 실제 socket EOF를 읽고 검증했으며 peer.stop으로 대체하지 않았다.

T030–T033 체크는 이 controlled 범위다. T034–T038 actual exact20fcd5f binary/private-root/aw subprocess 및 최종 affected/root8 checks·순차 OCR→Codex review·PR/CI/merge/main sync·인계 문서는 미완료다. T016 production exit6은 미충족 prerequisite 이연으로 최종 리뷰 대상이며 production gate 유지. 045/046·TUI/MCP·배포·fallback 후속 구현 시작0. goal.md와 사용자 docs2개 stage/commit0.

## 2026-09-29: T034–T039 actual merged044 서버 및 CLI wire checkpoint

기준 `27855e2` 이후 변경. `cargo fmt -p workbench-client -p aw-cli` exit0. `cargo test -p workbench-client -p aw-cli` exit0 **192passed/failed0/ignored1/filtered0**(client136/CLI56), 로그 `/private/tmp/aw-047-design/actual-wire-checkpoint-tests.log`. ignore1은 환경/명시 실행을 요구하는 actual_server이며 아래 별도 실행은1passed/ignored0이다. strict `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0(`/private/tmp/aw-047-design/actual-wire-clippy.log`). `bash -n scripts/test-workbench-client-wire.sh` 및 `git diff --check` exit0.

`cargo test -p aw-cli --test process_guard`의 현재 suite는 전체 실행에서 **5passed**. `/bin/sleep`의 startup deadline/pending ready future Drop, panic/error/explicit kill+wait, private root Arc lifetime, finite `/usr/bin/true`/exit deadline 및 pending test-only HTTP call wait timeout을 소유한 abort+bounded join/physical peer EOF로 검증했다. cleanup record는 pid/killed/reaped/status/error를 보존하며 실패 시 child를 Drop cleanup까지 유지한다. 실제 killed fixture는 kill(pid,0)=ESRCH/waitpid=ECHILD, normal probe는 exit/status/reaped를 확인했다. ready future의 temporary Path borrow compile exit101과 MutexGuard await/Drop match lint exit101은 lifetime binding/scoped guard/matches로 수정했다. cleanup 오류를 성공으로 덮거나 spawn된 child를 detach하지 않는다. 실제 실패 syscall 주입이나 macOS production process containment를 증명하는 시험은 아니다.

### 고정 서버 출처

`scripts/test-workbench-client-wire.sh --build-only` exit0. current branch source를 쓰지 않고 `git archive 20fcd5fdcf633ae06792d51a9b963e3857909440`을 `/private/tmp/aw-047-wire.dYwrsD/source`에 추출해 별도 target에서 `cargo build --locked -p agentic-workbench-server` exit0을 수집했다. build log/provenance는 해당 private directory에 있다.

| artifact | 실제 SHA-256 |
|---|---|
| server binary | `3fcb07ef711a77ffdc58f14c60f144455a7d4cfe3ec9cb5a83011fa67b425e9d` |
| exact source archive | `d6366d5165e683797499b5f82ca0db4402896aae82cb9faf54bb6f9c0b4a19ed` |
| baseline Cargo.lock | `85689f1fd6c7eef6f3152a3582d648ebbfcd1391c5fcf5a273c555c78b09fd4a` |
| copied actual aw under test | `d5631cba2397e28d50dddde3ea7dae1aeaeb2226ac7ac947b56fb2efccfe8d89` |

host macOS15.6.1 arm64. baseline server package version는 변경0이며 현재 branch binary를 old server로 위장하지 않았다. 사용자 branches/worktrees/source·docs2개는 보존했다.

### 실제 실행

실제 명령: `AW_047_SERVER_BINARY=/private/tmp/aw-047-wire.dYwrsD/target/debug/agentic-workbench-server AW_047_SERVER_PROVENANCE=/private/tmp/aw-047-wire.dYwrsD/provenance.json AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture` exit0 **1passed/failed0/ignored0/filtered0**. 로그 `/private/tmp/aw-047-design/actual-wire-progress.log`, 안전한 event/ordinal/sha 증거 `/private/tmp/aw-047-design/actual-wire-evidence.json`. combined 시험은1회 명시 server `serve --data-dir private`를 시작하며 자동 ensure0이다.

- 실제 readonly descriptor(pid=owned child), fresh identify→same TCP authenticated handshake 및 library/CLI system.describe/project.list parity, actual project create/update/delete와 동일 key library/CLI replay(true)/effect1·final list[]를 확인했다. CLI의 별도 invocation별 private caller state-dir를 사용하며 state cache로 HTTP replay를 대신하지 않았다. 기존 completed key state를 최초 publish로 덮는 잘못된 fixture는 privateState5로 실패(exit101)했고 새 caller root로 분리했다. `--retry-state` Unknown 복구는 기존 controlled abrupt-exit proof와 구분한다.
- bench.open→orchestration.bootstrap 반환 `eventStreamId`와 benchId/workspaceId가 구별되며 두 실제 WS consumer의 epoch/stream/schema/workspaceId가 일치한다. Main1/currentRunId null/assigned task null/active generation null, generations/tasks/reports/commands/coordinatorNotifications/dispatch0, bench runs[] 및 business gate reservations/busy/queued/tasks/pending operations/notifications0을 bootstrap 후·recover 직전·완료 후 확인했다. nonempty면 private recover를 제출하기 전에 assertion 실패한다. production CLI generic recover는 prerequisiteUnavailable8을 실제 검증했다.
- actual Rust EventSession 독립 consumer와 sandboxed 실제 aw subprocess에 cursor0을 전달하고 bootstrap JSONL/consumer ACK 이후 test-only HarnessConnection의 empty recover를1회 제출했다. bootstrap **sequence1/revision0**, runtimeReconciled **2/1**, notificationRecovery **3/1** 두 eventId/envelope를 exact 비교하고 same-revision event를 각각 ACK했다. HTTP complete output과 별도 orchestration.get final full snapshot revision1이 일치한다. actual 관측 ordinal은 event2=0/HTTP reply=1/event3=2였다. 이 ordinal은 HTTP 결과와 완전한 CLI JSONL 관측 순서이며 서버 전체 총순서로 확대하지 않는다. controlled 양방향 gate proof는 T032/T033에 별도 있다.
- SIGINT 뒤 CLI stream.end cursor3/exit130/stderr0/stdout EOF/bounded reap, 독립 session cursor3/cleanup error0, business run/task/dispatch0 및 owned server 최종kill/reap를 확인했다. 실제 server PID18413은 cleanup ledger reaped=true/error0 및 kill(pid,0)=ESRCH였다. host-side bounded owned ps 관측에서 해당 server의 child0을 확인했다. agents를 실행하는 operation은 제출하지 않았다.
- 동일 macOS seatbelt profile의 **private sentinel cat 성공(stdout exact/stderr0)**을 먼저 확인하고, 존재를 확인한 home AGENTS 파일 cat의 **권한 거절**과 shell background `/usr/bin/true` fork의 **권한 거절**을 대조했다. probe 모두 OwnedProcess의 exit deadline/kill/reap/cleanup record 안에서 실행한다. server와 copied SHA-identical aw는 user-home file access와 process-fork를 거절하는 profile에서 실행했다. 초기 nonzero+stdout0만 본 probe를 proof로 유지하지 않았다. 이 test-only profile/직접 child 관측/empty runtime proof는045 production containment, signed installed binary, 실제 ACP lifecycle 완료가 아니다.

실패/정정: 최초 actual query acceptedCalls0 기대는 실제1로 실패(exit101). merged044 `handlers/server::server_status`가 조회 자신을 포함한다고 명시하므로 현재 관측 호출 exact1/다른 accepted call0을 business launch reservation0과 구분했다. arbitrary >=/sleep으로 바꾸지 않았다. seatbelt 내부 ps의 KERN_PROC_ALL 실패(exit101)는 서버/CLI profile을 해제하지 않고 host-side owned observer로 분리했다. 단계별 실패는 별도 실행으로 남겼으며 최종 explicit1passed로 전체와 합산해 ignored 시험을 실행한 척하지 않는다. 최종 source 변화는 guard scoped formatting/lint 정리이며 재검증은 최종 gates에서 계속한다.

T034–T039 체크는 위 구현/증거와 한국어 `docs/workbench-rust-client-cli.md`/실제 quickstart 범위다. T040 affected checks/T041 OCR→Codex/T042 root8gate/T043 이연 인계 검증/T044 PR·CI/squash/main sync 및 최종 완료 인계 문서는 미완료. T016 exit6 및045/046·TUI/MCP·signing/update·desktop fallback 미완료 production gate는 유지하고 후속 구현을 시작하지 않는다. goal.md와 별도 사용자 docs2개 stage/commit0.

## 2026-09-29: 최종 workspace 및 OCR host 수정 checkpoint

`be9852c` 이후 client error의 full Fault를 Box로 보존하고, 초기 argv/signal 오류 출력에1초 deadline을 적용하며, Delivery public event 교체가 원 ACK를 통과하지 못하도록 queue Arc identity를 검사했다. actual stderr pipe를 WouldBlock까지 채우고 reader 미소비·parent 양 끝 열린 상태를 child reap까지 유지한 시험은 exit8/bounded reap를 확인했다. 잘못된 stream/epoch 또는 같은 identity·sequence의 다른 body로 교체한 token은 모두 StaleGeneration/queue·inflight·received·cursor 변경0이며 원 pending delivery는 reset 후 정상 ACK한다.

- `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --all-targets` exit0, **120 suites/1213passed/failed0/ignored8/filtered0/measured0**. 로그 `/private/tmp/aw-047-design/final-root-rust-tests-debug0-sequential.log`, target별 집계 `final-root-rust-test-counts.json`. 신규 client137/CLI57=194passed와 actual ignored1을 포함한다. 기존 scripted engine/core/host/Desktop 단위·integration 회귀를 production ACP/045 containment/desktop 설치본 matrix proof로 확대하지 않는다.
- `cargo fmt --all -- --check` exit0. `git diff --check`의 spec status trailing whitespace exit2는 해당 줄을 정정한 뒤 exit0.
- affected `cargo check -p workbench-protocol -p workbench-core -p workbench-host -p workbench-server -p agentic-workbench-server -p agentic-workbench` exit0 (`affected-rust-check.log`). `pnpm --filter @yoophi/workbench-client test` exit0 **9files/74tests/typeerrors0** (`affected-ts-client-tests.log`). protocol/lifecycle/TS consumer 구현 변경0, 기존 package version 변경0이다. frontend UI/FSD/Storybook·릴리스 version·Linux/Windows 변경/검증은 N/A이며 새 client/CLI production graph에는 host/core/server/Tauri0이다.
- `pnpm check-types` exit0 **13/13Turbo cached**, `pnpm test` exit0 **12/12cached**, `pnpm build` exit0 **5/5cached**. logs `final-root-{check-types,ts-tests,build}.log`. 새 frontend 실행으로 기록하지 않는다.
- `pnpm --filter @yoophi/workbench-client test:integration` exit0 **3files/7tests** (`final-client-integration.log`), `pnpm --filter @yoophi/agentic-workbench test:integration` exit0 **3files/14tests** (`final-aw-integration.log`). 기존 HTTP503 negative fixture 로그는 기대된 실패 응답이며 suite 실패0이다.

실패/재실행 이력: 최초 root strict Clippy는 ClientError::Fault variant128bytes 이상 result_large_err113건으로 exit101이었다 (`final-root-rust-clippy.log`). full fault fields를 Box 안에 보존한 뒤 workspace strict Clippy exit0 (`final-root-rust-clippy-boxed.log`); 최종 추가 수정 뒤 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --workspace --all-targets -- -D warnings`도 exit0 (`final-root-rust-clippy-debug0.log`). 개별 client Clippy와 workspace feature union의 크기 차이가 원인이라는 설명은 추론이며 관측은 root lint와 variant 크기다. 최초 root Rust test는 link ENOSPC exit101 (`final-root-rust-tests.log`). generated incremental 캐시 정리 후 default debug 재시도는 disk 여유가 다시 부족해지므로 owned cargo에 SIGINT를 보내 exit130으로 종료했다 (`final-root-rust-tests-boxed.log`), 제품 회귀 실패가 아니다. `cargo clean --profile dev` exit0/generated130747files·31.5GiB 정리 (`root-build-cache-clean.log`), source/user files/private baseline provenance는 보존했다. cleanup 완료 전에 debug0 build를 겹쳐 시작한1회는 제 실행 순서 오류로 syn object ENOENT exit101 (`final-root-rust-tests-debug0.log`)이었다. cleanup 완료 수집 뒤 **순차** 재실행이 위1213passed다. debug/incremental/jobs 설정은 생성 artifact·빌드 자원 설정이며 시험 대상/의미를 축소하지 않는다.

최종 strict Clippy·actual wire 재실행과 고정 HEAD/root8 evidence·순차 OCR→Codex 구현 리뷰·PR/CI/merge/main sync·완료 인계는 아직 완료 전이다. T016 production exit6 및045/046/TUI/MCP/signing/update/fallback 등 후속 gate는 유지한다.

최종 actual 재실행: 위 actual 명령에 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`를 적용하고 `AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-final-evidence.json`으로 실행, exit0 **1passed/failed0/ignored0/filtered0** (3.92s). 로그 `actual-wire-final.log`. exact server SHA/출처는 동일하고 새 CLI 바이너리 SHA는 final evidence에 별도 기록했다. actual bootstrap1/0→runtime2/1→notification3/1/ACK3, reply ordinal1/event ordinals[0,2], positive private sentinel/home 권한 거절/fork probe 거절, observerAccepted1/businessWork0/child0을 재확인했다. server PID28471 최종reaped=true/cleanup error0. 테스트 profile·empty fixture 근거이며 production containment/ACP launch 완료가 아니다.

T040 체크는194개 새 client/CLI를 포함한 root1213 및 affected/TS 근거다. T042는 review할 고정 HEAD에서 root8 gate를 재확인한 뒤 체크하며, 소스와 검증 artifact의 SHA를 연결한다. 후속 리뷰 수정이 있으면 해당 검증을 갱신한다.

## 2026-09-29: I-C1 notification lag/Shutdown 수정

고정 HEAD `fcae18e5494ec78ef3d88b6acc484228bc035662`의 root8은 전부 exit0이다 (`/private/tmp/aw-047-design/implementation-round1/frozen-root-gates.json`, 동일 directory의8logs). Rust1213passed/ignored8, TS Turbo13/12/5 cached, 실제 integration3files7tests 및3files14tests다. 그러나 같은 HEAD Codex 구현 verdict는 needs-attention High1이므로 merge하지 않았다.

- `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p workbench-client --test recovery same_epoch_notification` 수정 전 exit101/1failed(22filtered), gap이 snapshot live 경계 대신 이전 cursor를 반환했다 (`notification-gap-model-red.log`).
- 동일 env의 `cargo test -p workbench-client --test session notification_lag_and_shutdown` 실제 회귀 수정 전 exit101/1failed(16filtered), replacement verified hello 뒤 snapshot 진입 barrier timeout (`notification-gap-session-red-runtime.log`). outer panic을 수집한 뒤 retained OwnedTask abort+bounded join/physical peer EOF를 확인하고 원 assertion을 다시 전파했다. 최초 helper 이동의 Duration import compile101 및 super::task 경로 compile101은 별도 실패 로그에 보존했다.
- 같은-epoch SubscriberLagged와 Shutdown은 non-retaining stream일 때 새 generation의 live-first snapshot/reset round를 시작한다. boundary는 gap last와 received의 최대값이며 기존 applied와 구분한다. round의 모든 consumer reset이 끝나기 전 delivery0, retained stream의 cursor replay/독립 consumer 진행은 유지한다.
- `cargo test -p workbench-client -p aw-cli` 동일 env exit0 **196passed/failed0/ignored1/filtered0** (client139/CLI57), `notification-gap-fixed-tests-final.log`. recovery23/session17이다. model은 두 consumer에 live-only event3을 buffer한 뒤 하나만 reset 완료해도 delivery0/applied1, 모두 reset2 뒤 event3/ACK3을 확인한다. actual socket은 disconnect·lag·Shutdown 모두 과거 notification2 미재전송을 명시하고 live3만 보낸다. verified hello 이후 snapshot pending 및 reset pending 동안 buffered item 존재/consume0/ACK0/applied1, reset2 후 event[1,3]/ACK3 및 queue0/cleanup0/peer EOF를 확인했다. gap ticket cursors[0,2], disconnect[0,1]이다.
- `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` 동일 env 최종 exit0 (`notification-gap-clippy-final.log`). helper를 두 module로 로드한 초안은 duplicate_mod exit101이었다 (`notification-gap-clippy.log`); 원 shared harness의 OwnedTask를 그대로 재사용하도록 정리했으며 별도 중복 module을 남기지 않았다. fmt exit0/diffcheck exit0.

실제 서버·최종 root8·전체 OCR→Codex를 수정 HEAD에 다시 묶어 검증한다. 원 needs-attention을 승인으로 바꾸거나 기존 fcae18e root8만으로 이번 수정 검증을 완료했다고 하지 않는다.

I-C1 이후 actual 명령은 앞선 동일 baseline/env로 `AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-notification-gap-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture`, exit0 **1passed** (3.77s), `actual-wire-notification-gap.log`. 새 CLI SHA `899d702558423a3d3debc171cf03d8e9421cf2cbb6b41af8df82ce706fea265a`, server SHA `3fcb07ef711a77ffdc58f14c60f144455a7d4cfe3ec9cb5a83011fa67b425e9d`. 실제 vector1/0→2/1→3/1/ACK3·snapshot1, probes 대조 및 serverPID16674reaped/child0을 재확인했다.

## Codex 구현2 Medium 수정 checkpoint (2026-09-29)

OCR73/73(skipped0) 뒤 fa4c42f의 순차 Codex thread01a0ed40-3d1d-7132-9407-9e7c10815656, exec14405 실제exit0는 **needs-attention Medium2**였다. artifacts `implementation-round2/`에 원 결과와 fa4c42f root8 all exit0(workspace1215passed/ignored8)를 보존했다.

- I-C2: 완료 reply와 원 input을 HTTP body8MiB 상한에 함께 저장하여 Applied를 Unknown으로 남길 수 있었다. 별도 RetryState256MiB 저장/읽기 상한과 publish-before-submit reserve를 적용했다. 예약은 현재 원 input/identity 직렬화 크기 +24×raw body 최대치 +64KiB이며 overflow/부족은 전송 전에 거절한다. 24배는 arbitrary_precision 없는 i64/u64/finite f64 최대24bytes, 문자열 raw byte당 escape 최대6bytes, 구조 구분자 유지에 근거한다. raw와 normalized 길이가 같다는 가정은 제거했다. exact8MiB reply+nonempty input은 terminal cache로 reopen하며 실제 aw 재호출은 requests3/effect1, 동일 stdout/추가HTTP0이다. 지수표기1e10 reply/fault를 body64KiB 경계에서 parse→serialize하면 raw+64KiB보다 커지지만 완료 저장/reopen Applied 캐시를 보존한다. 실제 raw HTTP 지수 reply subprocess도 normalized state>raw+64KiB, reopen 동일 stdout/추가HTTP0을 확인했다. f64 지수−324..308·부호·mantissa 및 integer extrema는24bytes 상한 회귀를 포함한다.
- I-C3: 활성 watch 동안 O_NONBLOCK 공유 OFD lease를 호출자 계약으로 명시했다. 부모 concurrent writer는 WouldBlock 처리/flags 변경 금지, machine JSONL은 전용 출력 사용이 필요하다. 종료 후 원 flags 복구와 활성 기간 flags 불변을 혼동하지 않는다. 실제 shared Unix socket subprocess는 원 blocking/nonblocking × SIGINT/protocol exit4경로에서 부모 쓰기 성공→burst WouldBlock→전 bytes drain→자식 종료→exact 원 flags 및 부모 재사용→실제 socket EOF를 확인한다. regular file/devnull은 lease 없음, regular write64KiB는 byte cap만 의미한다.

실제 환경은 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`이다.

- `cargo test -p workbench-client -p aw-cli` exit0 **202passed (client143/CLI59), failed0, ignored actual1**, `review2-corrections-tests-retry.log`. 이전 exact8MiB red는 Body exceeded exit101(`retry-state-budget-red.log`); 정규화 fault fixture의 requestId 불일치1회는 stale completion exit101(`review2-corrections-tests.log`)였으며 원 requestId를 그대로 사용하도록 고쳐 재실행했다.
- `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0, `review2-corrections-clippy.log`; `cargo fmt -p workbench-client -p aw-cli` exit0; `git diff --check` exit0.
- `AW_047_SERVER_BINARY=/private/tmp/aw-047-wire.dYwrsD/target/debug/agentic-workbench-server AW_047_SERVER_PROVENANCE=/private/tmp/aw-047-wire.dYwrsD/provenance.json AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-review2-corrections-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture` exit0 **1passed**(3.77s), `actual-wire-review2-corrections.log`. exact20fcd5f serverSHA3fcb07ef…b425e9d, bootstrap1/0→runtime2/1→notification3/1 ACK3/snapshot1, positive private sentinel/negative home/fork probes, serverPID8284reaped/child0。이는 fixture 증거이며045 production proof가 아니다.

새 고정 수정 HEAD에서 root8 및 OCR→Codex 전체 순차 재리뷰는 다음 단계다. T041/T042 및 PR/merge/인계는 아직 미완료이며 production RunCancel/recover 및045/046 gates는 유지한다. SSH fetch timeout exit128 및 bounded HTTPS fetch30초 transport timeout(owned process SIGTERM/exit−15), 부모 gh TLS handshake timeout을 관측했다. 인증/remote 변경0이며 permission 문제로 단정하지 않는다.

## Codex 구현3 terminal snapshot 수정 checkpoint

고정 `be63aafc8a97e2932d521ead70bf5929deda1e4e`의 root8 gate는 모두exit0였다: `cargo test --workspace --all-targets` **120suites/1221passed/failed0/ignored8**, workspace strict Clippy/fmt, `pnpm check-types/test/build`, TS client integration3files7tests 및 AW integration3files14tests. full frozen manifest/log와 OCR73/73(skipped0)→Codex3 원 결과는 `implementation-round3/`에 보존했다. Codex thread01a0ed4a-bd70-76b2-84ef-66756d4e814a /exec16577 actualexit0, verdict **needs-attention Medium1 I-C4**, approve가 아니다.

I-C4는 snapshot HTTP의 Identity/Incompatible/Protocol/nonretryable Fault까지 재시도한 뒤 Unavailable로 덮는 유효 지적이다. complete_snapshot은 owner/generation/scope `check_load`를 먼저 실행한다. 이전 stream generation/listener scope/foreign recovery owner의 늦은 Identity/Protocol은 stale이며 새 Live cursor6→ACK7에 영향0이다. current terminal은 기존 close로 listener/round/reducer pending을 무효화하고 원 error/fault 모든 필드를 반환한다. connect와 snapshot은 동일 transient allowlist(Unavailable/Deadline/TransportUnknown 또는 retryable unavailable/draining/rateLimited/deadlineExceeded Fault)를 사용한다.

- 최종 `cargo test -p workbench-client -p aw-cli` exit0 **207passed/failed0/ignored actual1** (`terminal-snapshot-tests-final.log`). final close 재사용 후 `cargo test -p workbench-client --test recovery` exit0 **26passed** (`terminal-snapshot-model-green.log`). 기존 함수로 되돌려 같은 terminal regression을 실행하면 exit101/terminal must not reconnect (`terminal-snapshot-behavior-red.log`), finally로 수정 source를 복구했다.
- 초기 시험 작성은 non-Clone LoadRequest 복제 시도 E0599/exit101 (`terminal-snapshot-red.log`) 및 private SnapshotCompletion fields 접근 E0451/exit101 (`terminal-snapshot-tests.log`)로 실패했다. 제품 token API를 공개하지 않고 실제 owned spawn_snapshot→join으로 completion을 얻도록 고쳤다. 이후 listener exhaustion의 old fixture가 Protocol을 transient failure로 사용한1failure/exit101 (`terminal-snapshot-tests-retry.log`)를 발견했다. fixture의 의도인 transient Unavailable로 수정했고 terminal protocol 별도 matrix는 유지했다.
- model terminal/transient/old-token3개 및 actual WS session matrix1개 추가: terminal Identity/Incompatible/Protocol/auth Fault는 loads1/추가snapshot0/connect[0,5]/ACK0/reset0/consume0, transient Unavailable은 loads2/connect[0,5,5]→reset5/ACK6. outer timeout/panic 뒤 OwnedTask abort+bounded join 및 실제 peer EOF, cleanupError 없음/queue0을 확인한다.
- 실제 aw snapshot HTTP auth Fault 및 malformed protocol matrix1개: 성공한 stream.open 후 gap/재연결 hello→snapshot failure→원 stream.end unauthenticated/protocolViolation와 exit3/1, calls1/tickets2/cursor0/추가snapshot0, stderr0. 실패·panic·timeout도 child 소유권을 유지하여 bounded kill/reap 후 peer EOF를 수집한다.
- `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0 (`terminal-snapshot-clippy.log`); cargo fmt exit0. `pnpm generate:contracts` exit0 및 `git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated` exit0 (`terminal-snapshot-contracts*.log`), 생성 계약 drift0. 환경은 앞 checkpoint의 CARGO_* DEBUG0/jobs2/incremental0을 유지했다.
- 앞과 동일 baseline/env, `AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-terminal-snapshot-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture` exit0 **1passed**(3.94s), `terminal-snapshot-actual.log`. serverPID97853reaped/child0, exact20fcd5f serverSHA3fcb07ef…b425e9d, actual CLI SHA`a2b159d96db262b0892b3bfc42c9895285255ec0a871d2b2df53381f5b8e2543`. bootstrap1/0→2/1→3/1/ACK3/snapshot1 및 private positive/home/fork 대조를 재확인했다.

수정 고정 HEAD의 root8와 OCR→Codex4 순차 재리뷰는 다음 단계다. T041/T042 및 PR/merge/인계는 미완료다. 네트워크 재확인: worker gh repo view exit0/ADMIN(0.59s), bounded HTTPS fetch exit0, 원격main exact20fcd5f. credential/remote 영구 설정 변경0.

## Codex 구현4 normalized snapshot/JSONL 수정 checkpoint

고정 `febc3267272417a1a976af05cbbc9e456ceded5e`의 root8 all exit0: workspace120suites/**1226passed/failed0/ignored8**, strictClippy/fmt, pnpm check-types/test/build 및 client/AW integration7/14tests. manifest/log와 OCR73/73→Codex4 원 결과는 `implementation-round4/`에 보존했다. Codex thread01a0ed53-7f71-7b41-b2c2-344026156ee1 /exec2168 actualexit0, verdict **needs-attention Medium1 I-C5**. near-limit reset wrapper로 유효 snapshot을 거절하는 지적은 유효하며 raw Body를 snapshot_loaded에서도 재사용한 부분을 함께 수정했다.

[표현별 예산 표](contracts/client.md#직렬화-예산-비교-i-c5)에 raw HTTP8MiB/WS1MiB·normalized snapshot192MiB·serialized event aggregate queue8MiB/256items·event/reset/error-end JSONL256MiB의 포함 범위를 대조했다. wire/frame/queue limit을 늘리지 않았다. Limits는 Snapshot≥24×Body 및 JsonlRecord≥max(Snapshot,QueueBytes)+2×Input+128KiB를 checked arithmetic으로 검증한다. cursor는 stream/epoch/keys 및 max u64 sequence 직렬화 크기를 Input1MiB에 예약하여 이후 sequence width 증가도 포함한다. construction/rebind/snapshot 및 JSONL open/reset/end 검사로 큰 metadata를 조용히 초과시키지 않는다. normalized snapshot와 최종 record size는 raw bytes와 같다고 가정하지 않는다.

- `cargo test -p workbench-client -p aw-cli` exit0 **212passed (client150/CLI62), failed0, ignored actual1**, `jsonl-snapshot-budget-tests-final.log`. 기존동작의 actual subprocess near8MiB regression은 stream.reset 대신 stream.end를 받는 assertion으로 exit101 (`jsonl-reset-boundary-red.log`,1.28s). 실패도 owned child kill/reap 및 실제 peer EOF 후 재전파했다. 수정 후 동일 exact8MiB HTTP body는 full snapshot reset5→live6/ACK6→SIGINT end6/exit130/stderr0, peer14requests 및 child/EOF 정리를 확인한다.
- raw exponent model은 raw array<64KiB가 normalize 후 Body보다 커져도 Snapshot budget에서 reset6→ACK7을 적용한다. JsonlOutput matrix는 raw exponent 성분, custom normalized snapshot 상한24KiB 및 serialized event queue 상한24KiB, cursor metadata max width reservation을 함께 사용하여 open/event/reset/error-end4개 완전한 JSONL을 검증한다. 이 matrix의 normalized snapshot은 representation 상한으로 padding한 합성 SnapshotPort 값이며 raw HTTP1KiB 전체 응답이라고 주장하지 않는다. 별도 actual HTTP snapshot은8MiB다. event record는 queue payload보다, reset record는 snapshot value보다 크지만 최종 wrapper/cursor/newline 포함 JsonlRecord 이하이고 성공 write/flush 후 cursor를 적용했다. limits의 exact serialized cursor1MiB/metadata+1 및 부족 config/overflow guard·모든9 Resource exact limit/+1은 별도 회귀다.
- `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0 (`jsonl-budget-clippy.log`), cargo fmt 및 git diff --check exit0. 환경은 CARGO_INCREMENTAL=0/CARGO_BUILD_JOBS=2/CARGO_PROFILE_DEV_DEBUG=0/CARGO_PROFILE_TEST_DEBUG=0이다.
- 동일 exact baseline/env, `AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-jsonl-budget-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture` exit0 **1passed**(3.79s), `jsonl-budget-actual.log`. actualCLI SHA`12ea8748d0930af5f5848fe579ca48f441e36a2885ca6120c9c170d00c6b6901`, exact20fcd5f serverSHA3fcb07ef…b425e9d, serverPID87821reaped/child0. bootstrap1/0→2/1→3/1/ACK3/snapshot1 및 positive private sentinel/negative home/fork fixture 대조를 재확인했다.

수정 HEAD의 root8와 OCR→Codex5 순차 재리뷰는 다음 단계이며 T041/T042·PR/merge/인계는 미완료다. 045/046/TUI/MCP/signing/update/fallback 후속 구현0·gate 유지다.


## Codex 구현5 Evicted/listener snapshot backoff checkpoint

be82f56f9d6386b53d8072c7d7e5168e6684bc20 고정 root8는 all exit0: workspace120suites/1231passed/failed0/ignored8, strictClippy/fmt, pnpm check-types/test/build 및 client integration7/AW integration14tests. 원 frozen manifest/log와 OCR73/73(skipped0)→Codex5는 `implementation-round5/`에 보존했다. Codex thread01a0ed5f-713a-7611-8492-1a250e4f0332 /exec20855 actualexit0는 needs-attention Medium1 I-C6였다. Evicted의 직접 snapshot 실패는 reconnect backoff를 지나지 않아 예산을 즉시 소진했고 listener도 같은 경로였다.

두 직접 load 경로의 첫 시도는 즉시, 후속 시도는 attempt별 exponential/jitter backoff를 LoadRequest에 고정한다. sleep은 owned snapshot job 안에 있으며 actor는 계속 frame/gap/종료를 처리한다. 새 stream generation은 기존 cancel_callbacks abort+join을 사용하고 completion owner/generation/scope 검사도 유지한다. request timeout은 sleep 이후 시작하며 OwnedJob 전체 join 상한에는 delay를 포함한다. live-first Connect가 이미 backoff한 stream은 snapshot에서 중복 대기하지 않는다. attempt 수 증가/상한·동일 applied/live boundary·terminal cause 정책은 그대로다.

환경은 CARGO_INCREMENTAL=0/CARGO_BUILD_JOBS=2/CARGO_PROFILE_DEV_DEBUG=0/CARGO_PROFILE_TEST_DEBUG=0이다.

- `cargo test -p workbench-client --test recovery` exit0 29passed, `snapshot-backoff-model.log`. tokio test-util 가상시간으로 Evicted/listener × 성공/소진, 첫0/후속10s delay, delay−1ms 호출0, 정확히3loads/추가load0, 동일cursor, 취소중 추가call0/owned join 및 old delayed completion이 새 Live6에 영향0을 확인한다. dev-only test-util 추가이며 production trigger는 없다.
- sleep을 제거한 동일 model regression은 exit101/no retry burst(count2≠1), `snapshot-backoff-behavior-red.log`; finally로 수정 source를 복구했다. 초기 existing recovery도 exit0 (`snapshot-backoff-initial.log`).
- `cargo test -p workbench-client --test session actual_socket_eviction_and_listener_snapshot_backoff_success_exhaustion_and_cancel -- --nocapture` 최종exit0 1passed/6matrix, `snapshot-backoff-session-retry.log`. 실제 proof/ticket/WS/성공 opened 및 첫 port 진입 barrier 후 시간을 pause하여 Evicted/listener × 성공/3회소진/delay중취소를 검증한다. 호출 간 가상100ms 이상,99ms 시 추가call0·같은cursor0, 성공reset5, 실패/취소applied0, ticket1/재연결0·queue0·cleanupError없음, abort+bounded join 및 peer socket EOF를 확인한다. 첫 초안은 Tokio millisecond tick 반올림 때문에 정확100ms 후 count1≠2로 exit101 (`snapshot-backoff-session.log`); 실제 시간 sleep을 늘리지 않고 가상 tick1ms 여유를 추가했다. timeout/panic도 시간 resume→owned abort/join→EOF 후 재전파한다.
- `cargo test -p workbench-client -p aw-cli` exit0 **215passed/failed0/ignored actual1**, `snapshot-backoff-tests.log`; `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` exit0 (`snapshot-backoff-clippy.log`), cargo fmt exit0.
- 동일 exact baseline/env, `AW_047_WIRE_EVIDENCE=/private/tmp/aw-047-design/actual-wire-snapshot-backoff-evidence.json cargo test -p aw-cli --test actual_server -- --ignored --nocapture` exit0 1passed(4.12s), `snapshot-backoff-actual.log`. exact20fcd5f serverSHA3fcb07ef…b425e9d, CLI SHA62ac18eaa1e0df200ac55169435c01421a8a932bce01b4da8611bde76806e1c8, serverPID78584reaped/child0, bootstrap1/0→2/1→3/1 ACK3/snapshot1 및 positive private sentinel/negative home/fork fixture 대조.045 production proof로 확대하지 않는다.

새 수정 HEAD의 root8/OCR→Codex6 승인 전이며 PR/merge/인계는 미실행이다. T041–T044 미체크/production gates 및 최신047 중지범위 유지다.


## 최종 구현 source gate: f8df5dc (PR 전)

고정 source HEAD `f8df5dc4929c637c3799ac488849736fce76e890`에서 아래 root package.json 8개 명령을 모두 실제 실행했다. 환경은 위 CARGO_* DEBUG0/jobs2/incremental0이다. 로그 및 exact HEAD manifest는 `implementation-round6/frozen-root-*`에 보존한다.

| 명령 | 실제 결과 |
|---|---|
| `cargo test --workspace --all-targets` | exit0,120suites/1234passed/failed0/ignored8 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit0 |
| `cargo fmt --all -- --check` | exit0 |
| `pnpm check-types` | exit0,13tasks(캐시 포함) |
| `pnpm test` | exit0,12tasks(캐시 포함) |
| `pnpm build` | exit0,5tasks(캐시 포함) |
| `pnpm --filter @yoophi/workbench-client test:integration` | exit0,3files/7tests |
| `pnpm --filter @yoophi/agentic-workbench test:integration` | exit0,3files/14tests |

OCR host73/73/skipped0 후 Codex6 thread01a0ed67-c568-7ad2-b033-ebda44f85646 /exec91224 actualexit0, verdict approve/no material findings. 리뷰어는 read-only이며 시험을 실행했다고 주장하지 않았다. 기본215passed 및 actual baseline 별도1passed 및 앞선 근거는 위 checkpoint를 따른다. T016은 최신 사용자 scoped reachable 계약/닫힌 exit6 이연 및 parity 검토 완료로 체크하며 production exit6 실제 pass를 뜻하지 않는다.

이 최종 결과 기록은 source 검증 이후 문서만 갱신한다. 실행 소스/Cargo/lock/test/script bytes는 f8df5dc와 동일하게 보존하며 추가 문서 commit을 f8df5dc에서 직접 실행한8gate HEAD라고 위장하지 않는다. PR 최종 HEAD의 원격CI는 별도 수집한다. T043 최종 인계 연결/T044 실제PR·CI·merge/main sync/기록은 아직 미완료다. 045/046/TUI/MCP/배포/fallback 미실행·gate 유지다.


## 구현 PR/CI·squash·main sync 및 인계

[PR209](https://github.com/yoophi/agentic-workspace/pull/209) 최종head9947fbc38caa7fd3e9c72d778ea490239557dcc6의 [Quality run36577684368/job109437417847](https://github.com/yoophi/agentic-workspace/actions/runs/36577684368/job/109437417847) COMPLETED/SUCCESS, completedAt2026-09-29T14:01:40Z. worker connector 및 기존 gh 인증의 direct API 재검증에서 모든 step success를 확인했다. local f8df5dc root8 결과와 원격CI 최종head를 구분한다.

기본 API endpoint의 반복 TLS handshake/SSL timeout, GitHub browser navigation timeout, read-only connector merge403(Resource not accessible by integration)을 관측했다. 이를 사용자 ADMIN 권한 부족으로 단정하지 않았다. HTTPS DNS 조회로 얻은 다른 endpoint를 요청에만 curl resolve로 지정하여 certificate/hostname 검증을 그대로 유지했다. 기존 gh 인증을 stdout/argv/log에 노출하지 않고 process stdin config로 사용했다. expected head9947fbc와 CI success를 다시 검증한 뒤 squash API는 merged=true/`c3973ee5534850a66673867ce629b91ab3b67f9f`를 반환했다. 시스템 DNS/auth/remote/SSL 설정 변경0이다.

`git checkout main` 및 요청별 HTTPS URL rewrite/credential helper/Git curloptResolve를 사용한 `git pull --ff-only origin main` 실제exit0, fast-forward20fcd5f→c3973ee. `git rev-parse HEAD origin/main` 양쪽 c3973ee, `git diff --exit-code f8df5dc HEAD -- crates apps scripts Cargo.toml Cargo.lock` exit0/source bytes동일. 기존047 branch/044 worktree·045/046 branches 및 보호userfiles를 보존했다.

[최종 인계](../../docs/047-completion-handoff.md)에047 완료범위/commit·PR/증거와045/046·TUI/MCP·macOS14+/installed desktop matrix·signing/update·fallback 미완료, production gate 및 재개 방법을 기록했다. T016 production exit6 proof는 이연이며 fixture seatbelt·fork 대조를045 production proof로 계산하지 않는다. 전체 AW 전환 완료를 선언하지 않고 후속 구현0을 유지한다. 이 이후 변경은 완료 기록 문서뿐이다.
