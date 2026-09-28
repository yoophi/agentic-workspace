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
