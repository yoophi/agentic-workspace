# 045 설계 리뷰 반영 ledger

## 고정 리뷰 범위

- base: `20fcd5fdcf633ae06792d51a9b963e3857909440`
- reviewed head: `87c2fce8a5ccabbbcc179fb3626dc4ad77efe79d`
- 변경 파일: 11개 (`.specify/feature.json` 1개, Markdown 10개)
- OCR 자동 분류: total 11, reviewable 1, Markdown 10개는 `unsupported_ext` excluded
- OCR host 설계 검토: 11/11 실제 읽음, skipped 0, `needs-attention` (High 4, Medium 1)
- Codex adversarial: `review-mul90gfv-g9i2tc`, 11/11, `needs-attention` (Critical 1, High 4, Medium 2)

OCR 자동 분류 coverage와 host 수동 설계 coverage를 합쳐서 100%라고 표현하지 않는다. 위 두 수치를 별도로 유지한다.

## finding 판정과 반영

| ID | 출처 | 판정 | 중복 | 반영 |
|---|---|---|---|---|
| D1 | OCR High, Codex Critical | 유효 | 동일 근본 원인 | `Adopted → Published/Active/Aborting` 단일 CAS, loser typed result, ambiguous storage quarantine, 양쪽 winner fixture |
| D2 | OCR High, Codex High | 유효 | 동일 | Published/domain result/attempt-keyed outbox atomic transaction, reconnect replay와 projection dedupe |
| D3 | OCR High, Codex High | 유효 | 동일 | read-only helper의 transient domain 의미와 모든 ServerOwned child의 durable containment recovery anchor 분리 |
| D4 | OCR High, Codex High | 유효 | 동일 | live server의 keeper-exit 감시·즉시 cleanup 인계, combined crash startup reconcile, adoption/publication/cleanup 각 death fixture |
| D5 | OCR Medium, Codex High | 유효 | 동일 | checklist readiness PASS 해제, target spike 전 consumer migration 금지 |
| D6 | Codex Medium | 유효 | 없음 | GE/HL/MA production spawn source를 정확한 path로 inventory에 열거, app-wide wildcard 금지 |
| D7 | Codex Medium | 유효 | 없음 | protocol incomplete-frame deadline/minimum progress, slow-loris와 endless-valid-frame fixture |

## 남은 prerequisite 증거

- macOS/Linux: env-clear+exec descendant, actual process inventory permission, 재사용 안전 handle과 signal 원자성, keeper-only/server+keeper hard kill을 격리 spike로 검증한다.
- Windows: suspended create, Job assign/resume, breakaway denial, server crash kill-on-close를 실제 target에서 검증한다.
- 어느 target도 문서상 설계나 nonce 상속 fixture만으로 PASS 처리하지 않는다.
- prerequisite가 실패하면 전체 목표를 즉시 포기하거나 scope를 줄이지 않는다. 실패 target의 API/권한 근거와 대안을 새 설계 리뷰에 올리고 production consumer migration은 보류한다.

## T010 실패와 설계 재검토 입력

고정 증거는 CI run `36429844170`, HEAD `c3e292f91a39d522e38bfb1449c2d28c2d767324`이다. Windows는 명시적 `drop(job)` 시 미리 확보한 direct/descendant process handle의 bounded wait까지 실제 target에서 통과했다. 별도 owner/server hard-kill fixture는 아직 없으므로 Windows 전체 crash 계약 통과로 확대하지 않는다. macOS와 Linux job 자체는 exit 0이지만 required containment 판정은 실패다. macOS는 env-clear+new-session descendant가 nonce inventory에서 사라지고 ordinary parent가 arbitrary descendant audit token을 얻지 못했다. Linux는 pidfd가 exact known child signal을 보호했지만 env-clear descendant discovery를 제공하지 않았고, runner cgroup v2 subtree 생성은 `EACCES(13)`였다. PID 재사용을 실제 유발하는 identity-check/signal race fixture도 아직 없어 T005는 미완료다. 따라서 T010은 미완료이며 production consumer migration을 시작하지 않는다.

### 검토할 배포 대안

| 대안 | 충족하려는 불변식 | 배포/권한 비용 | 다음 결정 증거 |
|---|---|---|---|
| macOS 11+ 표준 Endpoint Security client를 포함한 signed system extension | 전역 fork/exec/exit event를 attempt root부터 필터링해 env와 무관한 subtree identity 유지 | Apple 승인 entitlement, Developer ID/provisioning, `/Applications` 설치, 사용자 system-extension 승인과 Full Disk Access, privileged/TCC 운영 검토 필요 | macOS 15 release-shaped artifact에서 event 누락/재연결, env-clear/session/double-fork, extension/서버 crash recovery와 exact signal을 검증 |
| macOS 27+ Endpoint Security descendant client를 포함한 signed system extension/broker | fork/exec/exit 전체 subtree identity와 audit token을 env와 무관하게 유지 | Apple 승인 `com.apple.developer.endpoint-security.client`, system extension signing/activation, macOS 27+ 필요 | 실제 entitlement가 있는 release-shaped artifact에서 subtree event, env-clear/session/double-fork, broker/서버 crash recovery, exact signal을 검증 |
| macOS ordinary app/keeper 유지 | 설치 비용 없음 | 현재 증거로 required containment 불충족 | 채택 불가. process group/nonce만으로 성공 처리하지 않음 |
| Linux systemd service/scope의 delegated cgroup v2 | launch 전에 attempt cgroup을 만들고 birth-time placement로 모든 descendants를 kernel membership에 두며 `cgroup.kill`/events로 quiescence 판정 | system/user unit 또는 D-Bus transient unit과 `Delegate=yes`, `clone3(CLONE_INTO_CGROUP)` 또는 payload instruction 전 membership이 증명된 동등 경로 필요 | supported distro의 release 설치 경로에서 spawn→move gap 0, env-clear/double-fork, daemon crash/startup recovery, direct launch fail-closed 검증 |
| Linux privileged containment broker | systemd가 없거나 user delegation이 없는 배포에서 cgroup subtree 소유 | 별도 privileged service 설치·업데이트·auth surface | 지원 distro/container matrix와 최소 권한 threat review 뒤 판단 |

Apple 문서는 Endpoint Security가 fork/exec 같은 process event를 제공하고 entitlement가 필요하다고 명시한다. 새 descendant-scoped client는 root/TCC 없이 전체 descendant subtree를 대상으로 하지만 availability metadata가 macOS 27.0+인 beta API라 현재 macOS 15 배포 해법으로 채택할 수 없다. Linux kernel cgroup v2 문서는 delegated subtree가 있어야 비권한 주체가 sub-hierarchy를 만들 수 있다고 정의하며, systemd `Delegate=`는 unit process에 그 하위 분할을 허용한다. 문서 가능성만으로 PASS 처리하지 않는다.

공식 근거(조회일 2026-09-28):

- Apple Endpoint Security: <https://developer.apple.com/documentation/EndpointSecurity>
- Apple macOS 11+ sample 설치/권한 절차: <https://developer.apple.com/documentation/endpointsecurity/monitoring-system-events-with-endpoint-security>
- Apple descendant client Markdown metadata (`macOS: 27.0.0 -`): <https://developer.apple.com/documentation/endpointsecurity/es_new_descendants_client%28_%3A_%3A%29.md>
- Linux kernel cgroup v2 delegation: <https://cdn.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html>
- systemd resource control `Delegate=`: <https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html>

### 후속 작업 의존성 판정

| 작업군 | 현재 상태 | 근거 |
|---|---|---|
| target feasibility, 배포 spike, 문서/설계 리뷰 | 진행 가능 | production consumer를 바꾸지 않으며 T010을 통과시키기 위한 작업 |
| pure lifecycle reducer, CAS/outbox storage transaction, output parser/policy의 isolated test/implementation | 설계상 분리 가능 후보 | OS process를 생성·adopt하지 않는 범위만 가능하지만 현재 tasks의 hard gate가 T011 이후 전체를 막으므로 재리뷰 전 시작하지 않음 |
| registry lease/adopt, recovery anchor, platform launcher | 금지 | viable containment adapter의 ownership·recovery semantics에 직접 의존 |
| ACP/terminal/Git/helper production migration | 금지 | macOS/Linux capability unavailable에서 child를 안전하게 소유·복구할 수 없음 |
| AW standalone server 전체 전환 | 계속 진행할 상위 목표 | 045 scope를 축소하거나 Windows-only 완료로 바꾸지 않음 |

OCR delegate는 위 대안의 제품 배포 가능성, platform-neutral foundation의 gate 분리 가능성, fail-closed UX를 먼저 검토한다. 그 지적을 반영한 고정 tree만 Codex adversarial `--wait`에 넘긴다.

## T010 실패 후 OCR 설계 재검토

- range: `c3e292f91a39d522e38bfb1449c2d28c2d767324..b357c7df22727e6c4a87bf7c2dc1cda519554dd3`
- OCR 자동 분류: total 4, reviewable 0, Markdown 4개 `unsupported_ext`
- host 수동 설계 coverage: 4/4, skipped 0
- verdict: `needs-attention` (High 2, Medium 1)
- 결과: `/private/tmp/aw-045-redesign-review/ocr-result.md`

| ID | 판정 | 반영 |
|---|---|---|
| O1 High | 유효 | macOS 27+ descendant API와 별도로 현재 macOS 15에서 검증할 macOS 11+ 표준 Endpoint Security system extension 후보, 설치·entitlement·TCC 비용을 명시 |
| O2 High | 유효 | Linux delegated cgroup에 `clone3(CLONE_INTO_CGROUP)` 또는 payload instruction 전 membership이 증명된 birth-time placement를 요구해 spawn→move escape를 금지 |
| O3 Medium | 유효 | 독립 후보를 pure reducer/storage transaction/output parser로 제한하고 registry lease/adopt/recovery anchor/platform launcher는 계속 gate 뒤에 유지 |

## OCR 반영 후 Codex adversarial 설계 재검토

- job: `review-mulavxtd-8s2aqb`
- range: `c3e292f91a39d522e38bfb1449c2d28c2d767324..6704ebd33c3def147c8b8b5460097bea841818ca`
- 실제 범위: redesign Markdown 4개와 inventory Python 2개, 총 6/6
- verdict: `needs-attention` (High 3, Medium 2)
- 별도 선행 code review `review-mulatfy5-hx9c0l`의 cfg(test) import finding은 `6704ebd`에서 먼저 반영했다.

| ID | 판정 | 반영 |
|---|---|---|
| C1 High | 유효 | comment/string delimiter를 lexical mask한 뒤 cfg(test) item 경계를 찾고 adversarial fixture를 추가 |
| C2 High | 유효 | T007/T009를 다시 미완료로 두고 macOS 15 release artifact와 Linux delegated birth-time placement actual jobs를 명시 |
| C3 High | 유효 | Windows를 Job API probe PASS/platform PENDING으로 하향하고 T006에 server hard-kill 등 남은 matrix를 명시 |
| C4 Medium | 유효 | T012–T015를 pure publication/outbox로 분리하고 containment anchor/schema/lease를 T016과 T010 뒤에 유지 |
| C5 Medium | 유효 | 최신 HEAD inventory 수를 5건으로 정정하고 새 exact HEAD matrix 재실행을 요구 |

## T011–T015 platform-neutral foundation 구현 증거

- T011 reducer는 `9672bb7`에서 publication winner와 published/active termination을 분리했고 단위 시험 6/6, strict Clippy를 통과했다.
- T012–T015는 containment·spawn·adopt·cleanup resolver를 참조하지 않는 typed publication port, SQLite v3 publication/result/outbox transaction, dispatcher 경계를 구현했다.
- publish/withdraw 경합은 한 store mutex의 순차 호출이 아니라 같은 file-backed DB를 연 두 독립 SQLite connection이 barrier에서 동시에 요청하는 시험으로 검증한다. `BEGIN IMMEDIATE` 뒤 조건부 `Pending → Published|Withdrawn` 전이가 winner 하나를 고정한다.
- commit-before-send는 store를 실제 close/reopen한 뒤 같은 result/event/payload가 `Replayed`이고 pending outbox가 하나인 것을 검증한다. commit 전 fault는 reopen 뒤 `Pending`/outbox 0, ambiguous-after-commit은 reopen 뒤 `Published`/정확한 replay/outbox 1을 검증한다.
- v2 file fixture를 v3로 migrate한 뒤 기존 operation ledger의 execution id, result JSON, contract revision과 schema handshake를 보존하고, v1→v3 연속 migration 및 future schema 거절도 유지한다.
- callback 전송 실패를 storage 실패와 구분한 typed delivery 오류로 반환한다. send 뒤 ack 전과 ack commit 뒤에 각각 store를 실제 close/reopen해 전자는 pending replay, 후자는 미전달 0과 ack 재시도 멱등성을 검증한다. 한 live `PublicationProjection` instance의 중복 억제도 별도로 검증하지만, 이 메모리 projection은 실제 client reconnect 또는 client process restart 보장이 아니다. WS replay/cursor와 실제 client projection 연결은 T020으로 미완료 유지한다.

foundation 구현 검증은 다음처럼 구분한다.

- pushed reducer HEAD `9672bb7c340d62db851149818026b0fff9e6b2ec`: GitHub Actions run `36431941843` 전체 success. 이 run은 이후의 미커밋 publication 변경 증거가 아니다.
- publication/result/outbox 구현 snapshot: `CARGO_INCREMENTAL=0 cargo test -p workbench-core --all-targets --all-features` exit 0, 576 passed / 0 failed / 7 ignored. 이후 typed delivery error와 file-backed ack 양쪽 fixture를 추가했다.
- 현재 publication 변경: integration 6/6, SQLite ledger unit 12/12, `cargo clippy -p workbench-core --all-targets --all-features -- -D warnings` exit 0, `git diff --check` exit 0. 전체 workspace/final gate는 구현 리뷰와 후속 045 작업 뒤 별도 실행한다.

## T012–T015 OCR delegate 구현 리뷰

- reviewed range: `9672bb7c340d62db851149818026b0fff9e6b2ec..752929b`
- OCR 자동 분류: total 11, reviewable Rust 6, Markdown 5개 `unsupported_ext` excluded
- host 실제 검토: Rust 6/6 + Markdown 5/5 = 11/11, skipped 0
- verdict: `needs-attention` (High 1, Medium 1)
- 결과: `/private/tmp/aw-045-foundation-ocr/result.md`

| ID | Severity | 판정 | 반영 |
|---|---|---|---|
| OI1 | High | 유효 | restart/retry의 `reserve → publish` 흐름이 기존 attempt에서 `DuplicateAttempt`로 막히지 않도록 `ReserveOutcome::Existing(record)`을 반환하고, Published result를 읽은 뒤 동일 publish가 replay되는 회귀시험 추가 |
| OI2 | Medium | 유효 | 현재 v3는 attempt당 logical publication 하나와 delivery ack만 저장한다고 data model을 정정하고, WS `stream_sequence`와 실제 reconnect/process-restart projection은 T020 경계로 명시 |

## OCR 반영 후 Codex adversarial 구현 리뷰

- job: `review-mulbyrp5-m9aj4c`
- reviewed range: `9672bb7c340d62db851149818026b0fff9e6b2ec..b2b53aaab6a0ccd29b9e086f8470290e1a91ac74`
- 실제 범위: Rust 6개 + Markdown 5개 = 11/11
- verdict: `needs-attention` (Medium 1)

| ID | Severity | 판정 | 반영 |
|---|---|---|---|
| CI1 | Medium | 유효 | arbitrary `event_kind`/`event_id`를 제거하고 `PublicationKind::{Accepted,Started}` 폐쇄 enum을 도입했다. event id는 attempt+kind에서 내부 생성하며 SQLite는 kind 허용값과 derived event-id 식을 모두 `CHECK`한다. raw invalid kind/ID와 기존 event 충돌이 publication을 Pending/result 없음으로 유지하고 outbox를 새로 쓰거나 바꾸지 않는 회귀시험을 추가했다. 기존 result/payload replay mismatch는 유지했다. |

Codex 반영 targeted 검증은 publication integration 7/7, SQLite ledger unit 14/14, strict Clippy exit 0, diff check exit 0이다. T016 containment/production migration과 T020 실제 reconnect/process-restart projection은 계속 미완료다.
