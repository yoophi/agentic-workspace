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
