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
