# Rust client/CLI 설계 근거

조회일2026-09-29, base20fcd5f. 아래는 actual source/API 선택 근거이며 실행 증거는 아니다.

## R1 독립 branch와 wire

Decision: merged044 기반, 045/046 cherry-pick 없음. protocol/lifecycle/TS client의046diff0. Rationale: client는 backup/process 구현을 조립하지 않고 기존 wire로 caller 계약을 시험할 수 있다. Alternative:046에 stack하면 미완료 backup 구현이 새 CLI delivery에 섞이므로 피한다. 다만 실제 production activation의045/046 dependency는 그대로다.

## R2 public full outcome

`workbench-host/src/lifecycle/calls.rs::call_envelope_by`는 output만 반환하고 Fault에서 code/message/status만 보존한다. `workbench-protocol/src/call.rs`의 CallRequest/CallReply와 `fault.rs::WorkbenchFault/Outcome`를 public caller model로 직접 사용한다. transport lost 뒤 Unknown을 유지하고 same key/payload/instance에 explicit retry를 묶는다. 호스트 helper를 그대로 wrap하는 대안은 replay/outcome/revision 손실 때문에 부적절하다.

## R3 loopback transport와 identity

원 `lifecycle/client.rs::verify_instance_by`, `descriptor.rs::read_descriptor`, `identity.rs::proof` 순서를 참고한다. nonce identify(credential 없음)→HMAC verify→credential handshake(instance/epoch/protocol/storage)→call. 원 HMAC은 SHA256(ownerToken) key, nonce newline instanceId payload다. client-only identity proof는 standard hmac/sha2를 쓰거나 pure shared helper로 추출하며 원 host 비교 vectors를 통과해야 한다. owner secret를 protocol serializer/Debug에 싣지 않는다. descriptor PID는 identity proof가 아니다.

HTTP는 existing lock reqwest0.12.28를 후보로 유지한다. [공식 pinned ClientBuilder](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html)의 no_proxy/redirect/total timeout 및 retry 정책을 확인했고 local installed0.12.28 source에 retry_policy/default retry가 존재한다. 실제 adapter는 `retry(reqwest::retry::never())`, redirect none、no_proxy를 명시한다. 이후 pinned0.12.28 공식문서를 조회해 API를 확인했다. `bytes()` whole-body 무제한 allocation 대신 bounded chunks+whole deadline. endpoint IP literal loopback/validated path만 허용, credential 전에 redirect/호환 검사.

## R4 WS와 applied cursor

existing lock tokio-tungstenite/tungstenite0.24.0. [공식 source](https://github.com/snapview/tungstenite-rs/blob/v0.24.0/src/protocol/mod.rs)의 WebSocketConfig max_message_size/max_frame_size는 adapter의 한도 후보. local pinned source·fake oversized fragmented frame regression으로 실제 동작 확인. TS `packages/workbench-client/src/event-client.ts` 및 reconnect/races/gap tests를 source-of-truth transition fixture로 사용하고 UI reducer를 복제하지 않는다. received와applied 분리, consume ACK 이후 cursor, live-first snapshot과 listener/connection operation generation, bounded backlog/close/cancel, hello만으로 gap복구 성공 아님.

## R5 auth/profile/production gate

원 owner authority와 agent scope는 다른 principal이다. 현재 agent capability로 owner HMAC를 검증할 수 있다고 가정하지 않는다. 별도 trusted endpoint/profile issuance 계약은 actual server support가 없으면 gated/unsupported. agent mode는 owner descriptor fallback0. readonly owner lookup은 private file evidence 검사 뒤만 허용한다. generic call은 production gate를 우회하지 못한다. 실제 launch/ensure/stop/배포/production data operations는045/046 readiness 증거 및 operation admission이 필요한 단계로 남는다.

## R6 finite command와 stdin

원 roadmap `docs/client-server-architecture-research.md` CLI/exit contract를 적용한다. status/query 및 catalog 먼저; input/action operation도 same library policy. failure outcome/unknown retry identity는 bounded diagnostic envelope에 보존하며 raw token/private input/details를 그대로 console로 dump하지 않는다. machine warnings는 envelope, log/stdout 분리. 원 `WorkbenchFault.details` 의미는 library에 보존하되 CLI의 diagnostic rendering은 closed safe fields를 별도로 투영한다.

미확정 실제 항목: ES activation/containment/046 migration, agent-profile trusted identity issuance, signed installed executable/update, concurrent actual desktop/TUI matrix. 이를 미해결 없음 또는 readiness PASS로 표현하지 않는다. 이들은 pure model/peer 구현을 막지 않지만 public production activation/전체 feature 완료를 막는 추적된 prerequisite다.


## R7 actual server integration source evidence

사용자 최신 요청으로 merged044 실제 binary/private root wire 경로를 포함한다. core `application/bench_service.rs::open`은 canonical directory→registry.open이다. 최초Main setPresentation 계획은 원 node!=Main guard로 불가능했으며 D-C1/R9에 수정 근거를 보존한다. 현재 actual event producer는 private empty fixture의 runtime.recover→service.reconcile_runtime→persist_mutation(runtimeReconciled,revisionr+1)이며 허용 상태/empty dispatcher branch를 아래R9에 명시한다. project CRUD에서 event를 emit한다고 가정하지 않는다. watcher에는 Git Command가 있어 actual event 경로로 사용하지 않는다. testserver fixture root bootstrap은 명시적 private scope이며046 live migration/freeze 우회가 아니다.

## R8 OCR 설계 반영: connection identity와 private retry state

D-O1 High 유효: reqwest의 일반 managed pool만으로 identify와 credential 요청이 동일TCP임을 보장했다고 주장할 수 없다. [공식 hyper HTTP1 handshake](https://docs.rs/hyper/latest/hyper/client/conn/http1/fn.handshake.html)와 [upgrade](https://docs.rs/hyper/latest/hyper/upgrade/index.html)를 사용해 sender/connection future/socket 수명을 직접 소유하는 adapter를 선택한다. 새TCP마다 unauthenticated identify부터, same socket에서 handshake/call 또는ticket WS upgrade. endpoint replacement fixture는 identity 성공 뒤 serverclose+다른 peer bind→credential0을 확인한다. standard HMAC verify_slice와 원host vector 비교를 사용한다. transport library 자체 retry없음、bounded read/deadline/task cleanup은 별도 시험한다. reqwest는 조사된 대안이며 현재 auth-bearing path 선택은 아니다.

D-O2 Medium 유효: CLI key-only retry는 previous payload/epoch bound identity를 보존하지 않는다. submission 전 owner-only private state에 immutable input/key/instance/epoch를 durable publish하며 failure전송0; 다음 invocation은 그 state의 동일 attempt만 재시도. 사용자입력→stderr receipt만으로 원source를 입증했다고 하지 않는다. caller runtime-control state는 서버 backup writer와 독립이고 실제 control-store 안전한 open/fsync/CAS/crash fixtures가 필요하다. client private state의 mutable active-use와 finalized outcome을 구분하고 old process completion이 새 attempt를 덮지 못하게 한다.


## R9 Codex 설계 High 반영: actual event producer

원 `service.rs::set_presentation`는 `node.id != MAIN_AGENT_NODE_ID`를 요구하므로 초기 Main-only fixture의 Main변경 계획은 불가능했다. 해당 계획을 실제통과로 쓰지 않고 수정한다. `runtime.rs::recover` 전체 함수와 `service.rs::reconcile_runtime/persist_mutation`, `command_service.rs::reconcile_pending`, `notification_dispatcher.rs::reclaim_orphaned/dispatch_pending_with`를 대조했다. recover는 일반적으로 side effect 있는 reconciliation이며 안전한 arbitrary workspace operation이라고 가정할 수 없다. 단일 private empty fixture(Main1/nullrun, all tasks/generations/commands/notifications0, in-flight0)에서 test-only trigger authority로 한정한다. service는 live_run_ids=[]로 reconciliation 후 revision을1올리고 runtimeReconciled event를 emit한다. command/reclaim loops empty, notification pass next_delivery=None은 worker queue 전에 종료한다. subscription hello+bootstrap event ACK 뒤 trigger를 보내 live runtimeReconciled event/sequence와 orchestration.get snapshot revision을 비교한다. presentationMain은 sourceNotFound로 보존하고 scoped test를 임의 child launch로 우회하지 않는다. public production recover gate는 그대로다.

## R10 Codex Medium 판별: revision과 stream sequence

review-mumgf0qh-iwt7yy의 비동기 후속 event 식별/ordering 누락은 유효하다. 다만 reviewer에게 emit_runtime_update_for body가 없어서 제시한 추가 revision 증가 가정은 원 코드와 다르다. runtime.emit_runtime_update_for는 service.get_for_bench readonly snapshot→emit_runtime_update이며 sink.emit은 payload revision을 그대로 hub.publish_state에 넘긴다. hub는 stream mutex에서 sequence만 증가시킨다. 빈 fixture에서 recover commit r+1/runtimeReconciled 이후 notificationRecovery도 r+1이며 sequence는 별개다. get_for_bench/persist_mutation/recover/spawn_notification_pass_with/두 emit/helper/sink/publish_state 전체를 후속 reviewer에 직접 제공한다. HTTP reply와 후속 event의 도착 순서 독립, 같은 revision event 보존, 두 reason exact 식별 및 ACK 후 최종 snapshot이라는 acceptance를 명시했다. 실제 wire 실행 증거는 통합 task에서 수집한다. 원 needs-attention verdict는 보존하며 source 판별을 시험 통과로 표현하지 않는다.
