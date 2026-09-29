# 047 설계/구현 리뷰 기록

현재 specify/plan draft, 구현·tests 없음. base20fcd5f의별도047 branch, 0456e4bf30·0469b1b2e2 보존. source dependency(protocol/lifecycle/TSclient) diff0 actual 확인. 046 partial7/31 및045macOS T010/T016 PENDING 유지. 새 Linux/Windows 구현/검증0. 관련 없는 untracked docs/code-review-app-migration.md 보존/제외.

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
