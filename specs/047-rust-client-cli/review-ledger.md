# 047 설계/구현 리뷰 기록

설계 시작 당시 이력: specify/plan draft, 구현·tests 없음. base20fcd5f의별도047 branch, 0456e4bf30·0469b1b2e2 보존. source dependency(protocol/lifecycle/TSclient) diff0 actual 확인. 046 partial7/31 및045macOS T010/T016 PENDING 유지. 새 Linux/Windows 구현/검증0. 관련 없는 untracked docs/code-review-app-migration.md 보존/제외.

주요 리뷰 질문: client-only dependency 경계、full fault/outcome/replayed 보존、unknown operation/key/epoch generation과 double send、descriptor/identity credential 순서、agent authority fallback、applied cursor/live-first recovery、quota/cancel/stdout、generic mutation productiongate。approved 순서는 OCR host manual 전체Markdown → Codex --wait이며 둘의scope/verdict/findings반영을 별도 기록한다. 그전 tasks/implementation 미실행.


## 선행 OCR delegate design

base20fcd5f→46a5ad5, 총10file(JSON1+Markdown9). OCR automatic reviewable1/excluded9unsupported_ext와 host actual manual10/10、skipped0/coverage100%를 분리했다. union은 실제git diff10과 정확히 일치/중복0. verdict needs-attention High1 D-O1(identity 뒤 새TCP credential 경합), Medium1 D-O2(CLI cross-invocation key-only retry bound identity 누락), 모두 유효. owned HTTP1 connection proof→credential/WSupgrade, 새socketproof와 private durable pre-send retry state/CAS·unknown reopening을 계약/plan/research/model에 반영했다. 이는 설계 반영이며 실행 시험 통과가 아니다. artifact `/private/tmp/aw-047-design/ocr-{preview,rules,host-review}.json`. 다음 고정tree Codex --wait에서 양 finding 해결과 전체 actual-server/nonlaunch gate를 검토한다. 구현 전.


## Codex design --wait: 실제event 경로 High

job review-mumg92ez-v6zw3v/thread01a0ec67-ffc1-72d1-9e00-fd19742a40c8, base20fcd5f/headdeb3f7c, actual tool69219 exit0/completed. verdict needs-attention High1 D-C1: Main-only bootstrap 뒤 setPresentation(Main)은 원 node!=Main guard에 의해NotFound라 post-subscription event 시험불가. 유효, 원 verdict 보존. reviewer는10변경file을 검토했고 이전D-O1/D-O2는 design에 반영됐다고 확인했으나 구현시험 pass 아님을 명시했다.

원 recover와service/reconciler/dispatcher 전체 경로를 대조해 empty private fixture에서만 test-only event trigger로 바꾸고 actual wire plan/contract/quickstart/research를 수정한다. input{benchId}/orchestration:write, streamorchestration:<benchId>/schemaorchestration.workspaceUpdated.v1, reasonruntimeReconciled, snapshot revisionr+1. CLIprojectCRUD 및 actualeventconsumer와 연계한다. worker/notification/command delivery는0 precondition, unexpected nonempty이면 trigger0/assert fail. 실제시험전이므로 결과를 만들어 기록하지 않는다. production generic recover는 gate유지. 수정설계 OCR→Codex 확인 뒤 tasks생성.


### D-C1 수정 선행 OCR delegate

base deb3f7c→b7600d0,5Markdown은 자동reviewable0/excluded5unsupported_ext. host가 전체5/5와 원 recover/reconcile/persist_mutation/command·notification empty branch를 실제 읽어 reviewed5/skipped0/manualcoverage100%, 새material0. actualgit union5/중복0. active generation null의 next_delivery=None이 worker 전에 종료하고 service의 r+1/runtimeReconciled를 확인했다. 기존 sourceNotFound·High verdict와 productiongate 유지, 실행결과 아님. artifacts `/private/tmp/aw-047-design/ocr-correction-{preview,host-review}.json`. 수정tree Codex --wait에 전체10file와 원 전체함수 context를 제공한다.

### D-C2 수정: 후속 비동기 이벤트 계약

job review-mumgf0qh-iwt7yy/thread01a0ec6c-3c0c-7ad0-b515-e4441d195d1b, base20fcd5f/head640f1e7, actual tool91577 exit0/completed, needs-attention Medium1 보존. 후속 notificationRecovery event와 HTTP reply ordering 식별 누락은 유효; 추가 저장 revision 증가 추정은 emit_runtime_update_for/get_for_bench/emit/sink/publish_state 전체 대조로 기각한다. 두 event는 r+1을 공유하며 sequence만 각각 증가한다. contracts/client.md D-C2에 bootstrap s, runtimeReconciled s+1, notificationRecovery s+2 exact 식별·ACK, 두 이벤트와 reply 완료 후 최종 snapshot r+1, barrier 양방향 ordering을 명시했다. >=/sleep/제품 코드 변경으로 보완하지 않는다. 실행 시험은 아직 없고 SC-007 미완료. 전체 함수 context를 제공한 OCR→Codex 수정 설계 리뷰 뒤 tasks로 진행한다.

### D-C2 선행 OCR 및 Codex 실행 실패 (완료 verdict 아님)

640f1e7→7fde8ad의5Markdown 자동 reviewable0/excluded5unsupported_ext와 host 실제5/5·skipped0·coverage100%를 분리했다. exact Git union5/중복0, 새 material0, scoped design correction만 approve. artifacts `/private/tmp/aw-047-design/ocr-sequence-{preview,host-review}.json`. runtime emit 두 함수/get_for_bench/sink.emit/hub.publish_state 전체를 추가하여 Codex 직접 source context34섹션/115880bytes를 제공했다.

첫 --wait job review-mumgvmwq-bgwwu1는 actual tool32615/script exit1, verdict 없음. codex app-server startup은 ~/.codex SQLite state runtime 초기화 실패였으며 현재 managed 권한에서 해당 경로 write 불가를 관측했다. 허용된 private tmp에 mode0700 runtime state를 두고 기존 config/auth를 read-only symlink로 참조한 startup probe는 exit0. 원 설정/인증 contents 출력·수정 없음.

동일 고정 tree7fde8ad 재실행 --wait job review-mumgxfqd-q6hyxj/thread01a0ec79-605f-7ed2-bee6-d33e22e2548b, actual tool51315/script exit1/failed. workspace routing discovery failed로 유효한 structured review verdict 없음. network/routing 외부 실행 조건이 해결되어야 재개 가능하며 이를 새 design finding/approve/리뷰 완료로 계산하지 않는다. artifacts `/private/tmp/aw-047-design/sequence-retry-codex-{review.md,review.exit,source-context.md,source-manifest.json,focus.txt}`. 두 exec handle 종료를 수집했고 companion running0을 확인했다(현재 ps는 Operation not permitted). tasks/구현/시험/빌드 미시작, 제품 코드 변경0. 이전 D-C2 needs-attention은 보존하고 유효 부분은 설계 반영했으며 actual wire는 SC-007 미완료다. 다음은 같은 설계 source context로 Codex --wait 유효 결과 수집→valid findings 반영→speckit-tasks→speckit-implement 순서다. 전체047/045/046 및 전체 AW 목표는 미완료, production gates 유지.

### D-C3 재개: 실제 stream identity 수정

사용자의 실행 위치 변경 지시에 따라 Herdr w2:p1에서 재개했다. 이전 w1:p3는 pane_not_found이며 현재 pane의 Codex와 cwd를 확인했다. binding.rs::set의 독립 UUID, delivery_sink.rs::emit의 binding_id, orchestration_stream.rs의 반환 eventStreamId 구독 및 재바인딩 사례를 직접 확인했다. 계약의 orchestration:<benchId>를 bootstrap 응답 eventStreamId(내부 suffix는 bindingId)로 수정하고 plan/quickstart/model/research에 동기화했다. 위 D-C1 기록의 benchId suffix는 당시의 잘못된 설계 기록이며 이 D-C3가 대체한다. 원 verdict와 실패 기록은 보존한다. 실제 통합 시험은 아직 수행하지 않았다.

### D-C3 OCR → Codex 완료 및 D-C4 반영

base20fcd5f/head4a605da의 OCR automatic1JSON/excluded9Markdown과 host manual10/10(skipped0)을 대조했고 새 material0. artifacts `/private/tmp/aw-047-design/ocr-binding-{preview,rules,host-review}.json`. 후속 Codex thread01a0ec90-7b21-7fb2-a602-8c06f9cfeb93, actual exec15810 exit0, verdict needs-attention Medium1. 전체10파일 검토에서 실제 orchestration 이벤트를 소비할 CLI 명령 부재를 지적했다. 유효: run watch만으로는 실제 aw subprocess JSONL/ACK 증거를 만들 수 없다. `aw events watch --input -`의 exact cursor schema와 반환 eventStreamId 구독, bootstrap 출력 뒤 recover, 두 이벤트 JSONL/ACK 및 bounded SIGINT/reap를 spec/CLI contract/plan/quickstart에 반영했다. artifacts `/private/tmp/aw-047-design/binding-codex-{review.md,review.exit,source-context.md,source-manifest.json}`. 설계만 수정했으며 구현/시험은 미시작이다. 다음은 이 수정의 OCR → Codex 재검토다.

### D-C4 再리뷰 완료 및 tasks 생성

OCR 4a605da..bd6099c automatic0/excluded5Markdown, host manual5/5(skipped0) 새 material0. Codex thread01a0ec91-d009-7750-a652-3dde6733f0d5 / exec76973 actual exit0, verdict approve. 리뷰어는 bd6099c 전체 설계에 새 material finding 없음, CLI subprocess 경로가 D-C4를 설계상 해결했다고 명시했으며 implementation/actual acceptance 미실행과 045/046/TUI/MCP/signing gates를 보존했다. artifacts `/private/tmp/aw-047-design/watch-codex-{review.md,review.exit,source-context.md,source-manifest.json}` 및 `ocr-watch-{preview,host-review}.json`. 이후 setup-tasks.sh --json으로 지정 template/context를 확인하여 44개 tasks를 생성했다. extensions.yml 없음. 모든 task는 미완료에서 시작한다.

## 최신 범위 변경: 047 완료·인계 후 중지 (2026-09-29)

사용자 지시로 기존 전체 전환 완료 기준을 supersede했다. 047 자체 가능한 계약·exact merged044 actual server/aw subprocess 검증·OCR delegate → Codex adversarial --wait 순차 구현 리뷰 및 수정·재검증·PR/CI·squash merge·main checkout/pull·완료 인계 문서 후 중지한다. 045/046 보완·TUI/MCP·배포·desktop fallback 제거는 후속 구현을 시작하지 않고 미완료/gate 유지/재개 조건을 인계한다. T016 production cancel-rejected exit6은 prerequisite 종속 이연으로 명시하여 검토받으며 현재 production gate를 해제하지 않는다. T043의 전체 roadmap 완료 요구는 이연/gate/인계 정확성 검증으로 대체됐다. 이는 구현 리뷰 approve나 가능한 047 시험 면제가 아니다. 수정된 spec/plan/tasks/validation 및 cli exit 계약을 최종 구현 순차 리뷰 범위에 포함한다.

## 구현 OCR host preliminary 검토 및 수정

base20fcd5f/headbe9852c의 deterministic preview는73files/automatic reviewable60/excluded13이다 (`implementation-ocr-preview.json` 및 `implementation-ocr-rules.json`). host는 Rust/client·CLI·실제/controlled 시험·shell·Cargo/lock·전체047 산출물을 직접 읽어 검토했다. Markdown/lock의 자동 제외를 검토 생략으로 계산하지 않는다. 고정 수정 HEAD의 최종 preview/coverage/report를 생성한 뒤 Codex를 순차 실행하며, 현재는 Codex 구현 approval 전이다.

- I-O1 Medium: main의 argv parse/signal 등록 오류 경로는 caller deadline 이전 finite stderr를 무기한 기다릴 수 있었다.1초 diagnostic deadline과 실제 full pipe/reader 미소비/parent FD 열린 상태/exit8·bounded kill/reap 회귀를 추가했다. workspace Rust 전체에서 해당 회귀 통과.
- I-O2 Medium: 고유 reducer owner/generation이 일치하는 Delivery의 public event를 같은 sequence의 다른 payload로 바꾸면 원 pending event를 ACK할 수 있었다. queue front의 원 Arc identity 검사와 stream/epoch/body 교체 후 상태 변경0·원 event 재소비 회귀를 추가했다. workspace Rust 전체에서 해당 회귀 통과.
- workspace root strict lint의 large ClientError는 full WorkbenchFault를 Box로 보존해 수정했다. JSON/details/outcome/retryable/requestId를 지우거나 코드로 축약하지 않았고 root1213tests 및 최종 strict Clippy exit0이다.

일반 reqwest pool/fresh TCP proof 경합, execute future Drop Unknown, Arc store CAS, retryable fault reopen, restore preflight Unknown, cancel typed 입력/gate parity, unfinished stdin barrier, 최초 publish syscall 경계, production hook 격리 panic, reducer owner/replay interleave, epoch/exhaustion stale completion, pending opened/budget/transient reconnect/full reset retry budget, stdout shared OFD/devnull/EOF, recover request 수신 이전 이벤트 fixture, sandbox probe false positive 및 owned deadline에 대한 사용자 검토 지적을 구현/회귀와 validation의 각 checkpoint에 반영했다. pending external prerequisites는 false PASS로 바꾸지 않는다.

### 구현 Codex1: I-C1 High 및 Shutdown 보완

headfcae18e/base20fcd5f의 OCR73/73·skipped0 후 Codex adversarial `--wait --model gpt-6-sol`을 순차 실행했다. thread01a0ed38-8e0b-7c21-aa6c-603afe6a2910, 실제 exec51744/exit0, verdict **needs-attention High1**. 원 결과는 `/private/tmp/aw-047-design/implementation-round1/implementation-codex-review.md`에 보존했다. exit0을 approve로 해석하지 않는다. reviewer는73-file diff와 원 event hub를 읽고 notification SubscriberLagged shortcut의 replay 불가로 인한 손실을 지적했다. 유효하다. 사용자 후속 지시에 따라 같은 분기의 Shutdown도 함께 수정한다.

non-retaining stream은 gap last/received 경계에서 새 live-first round와 generation을 시작하며 fresh hello 이후 snapshot/reset을 수행한다. 모든 stream reset 전 delivery를 막는다. retained replay·느린 reset과 다른 consumer 독립 진행은 보존한다. regression은 과거 notification2를 실제로 재전송하지 않고 live3만 보내며 pending snapshot 및 pending reset 동안 consume/ACK0·applied1을 확인하고 reset2→live3/ACK3로 종료한다. model은 두 consumer 중 하나만 reset한 경우에도 delivery0을 확인한다. failure/panic/outer timeout 뒤 owned abort+bounded join 및 actual peer EOF를 수집한다. task guard는 기존 ordering/process guard의 test-only Harness OwnedTask를 그대로 재사용한다.

수정 후 가능한 checks와 actual wire/root gates를 재검증하고 새 고정 HEAD에서 전체 OCR→Codex 순차 리뷰를 다시 실행한다. PR/merge는 그 전에 수행하지 않는다.

### 구현 Codex2: I-C2/I-C3 Medium 수정

fa4c42f 전체 OCR73/73(skipped0)→Codex thread01a0ed40-3d1d-7132-9407-9e7c10815656 /exec14405 actualexit0, verdict **needs-attention Medium2**. 원 artifacts `implementation-round2/`를 보존했다. I-C2는 유효한 큰 reply 완료를 body-budget state에 저장할 수 없는 결함으로 판단했다. RetryState 별도256MiB와 input/identity+normalized reply reserve를 적용하고 exact8MiB nonempty input 및 raw exponent reply/fault reopen, actual aw 재호출 추가HTTP0을 검증했다. 사용자 보완대로 숫자 정규화 상한24배를 예약에 포함하며 raw body와 저장 JSON의 길이 동일성을 가정하지 않는다.

I-C3는 공유 OFD에 활성 O_NONBLOCK 효과가 있다는 유효 지적이다. reviewer가 제시한 대안 중 명시적 호출자 lease 계약과 실제 concurrent writer/backpressure 증거를 적용했다. active 공유 writer의 WouldBlock 처리/flags 변경 금지, JSONL 전용 출력 요구를 문서화했고 실제 subprocess4경로에서 parent write/burst WouldBlock/drain/종료 exactflagsrestore/재사용/socketEOF를 확인했다. 종료 복구만으로 활성 기간의 공유 효과가 없다고 주장하지 않는다.

client/CLI202passed/actual 별도1passed/strictClippy exit0. 상세 actual 명령·수·exit 및 fixture requestId mismatch101는 validation에 기록했다. 새 고정 HEAD의 root8 및 OCR→Codex 재검토 전이며 이 기록은 최종 approve가 아니다.

### 구현 Codex3: I-C4 Medium 수정

be63aaf OCR73/73(skipped0)→Codex thread01a0ed4a-bd70-76b2-84ef-66756d4e814a /exec16577 actualexit0, verdict needs-attention Medium1. 전체 branch의 snapshot recovery가 terminal 신원/호환/프로토콜/nonretryable fault를 재시도하며 Unavailable로 덮는 유효 지적을 반영했다. complete_snapshot은 owner/generation/scope 검증을 오류 분류보다 먼저 실행하며 stale Identity/Protocol이 새 Live를 종료하지 않는다. 현재 terminal은 기존 close/invalidate 및 원 cause 반환, transient는 connect와 공통 allowlist/bounded retry를 사용한다.

model26/기본207passed, current terminal 및 transient actual WS matrix, 실제 aw HTTP snapshot auth/protocol error→원 code/exit·추가snapshot0·ACK0, owned cleanup/EOF 회귀를 확인했다. 기존동작 red101, typed private-token 시험 작성 실패와 old Protocol transient fixture 수정은 validation에 보존한다. strictClippy/fmt/contract generation drift0, exact baseline actual 별도1passed. 수정 HEAD root8와 OCR→Codex4 승인 전이므로 PR/merge는 아직 수행하지 않는다.
