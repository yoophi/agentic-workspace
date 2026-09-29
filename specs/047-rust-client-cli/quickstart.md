# 검증 안내 (아직 구현 전)

spec→plan→OCR→Codex→tasks→implement 순서. 이후 pure unit/golden/negative fixtures→controlled HTTP/WS peer→affected host/protocol conformance→strictClippy→필요한8gate. 이번 branch에는 아직 crate/binary가 없으므로 실행하지 않은 cargo 명령/통과 수치를 만들어 기록하지 않는다.

확인할 시나리오: 잘못된 identity/proxy/redirect credential0, malformed/oversize body/default-success0, response loss samekey/payload/1effect、epochchange0resubmit、delayed apply/gap/live snapshot race、stdin/argv/no-logs、SIGINT/localcancel/broken pipe/ACK0、agent owner fallback0、generic productiongate bypass0. actual ES/production restore/installed CLI/app/TUI matrix는 별도 PENDING.

base20fcd5f, 045/046 preserved refs 의존성은 plan을 따른다. 다음 구현 task는 순차 설계 리뷰가 완료한 뒤만 생성한다. unrelated user file은 stage/commit하지 않는다.


사용자 지정 actual server 통합: merged044 exact20fcd5f server build+SHA를 고정한 격리 fixture에서 client/CLI identity→handshake→project.list/system.describe, project CRUD same-key/replayed, bench.open→orchestration.bootstrap(empty)→WS subscription→empty-workspace recover event 수신·cursor/snapshot revision parity를 검증한다. actual ACP/child0, 사용자root0, cleanup완료가 acceptance다. server fixture child와 자동 daemon ensure를 구분한다. fakepeer→실제 wire→affected gate를 모두 기록하고 하나로 축소하지 않는다.

actual event acceptance는 contracts/client.md D-C2를 따른다. runtimeReconciled와 notificationRecovery의 같은 revision r+1/서로 다른 exact sequence s+1,s+2를 모두 소비·ACK한 뒤 최종 snapshot을 비교한다. HTTP reply 순서는 독립이며 barrier peer의 양방향 ordering 및 actual wire evidence를 별도로 기록한다.
