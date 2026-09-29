# Rust client/CLI 설계 근거

조회일2026-09-29, base20fcd5f. 아래는 actual source/API 선택 근거이며 실행 증거는 아니다.

## R1 독립 branch와 wire

Decision: merged044 기반, 045/046 cherry-pick 없음. protocol/lifecycle/TS client의046diff0. Rationale: client는 backup/process 구현을 조립하지 않고 기존 wire로 caller 계약을 시험할 수 있다. Alternative:046에 stack하면 미완료 backup 구현이 새 CLI delivery에 섞이므로 피한다. 다만 실제 production activation의045/046 dependency는 그대로다.

## R2 public full outcome

`workbench-host/src/lifecycle/calls.rs::call_envelope_by`는 output만 반환하고 Fault에서 code/message/status만 보존한다. `workbench-protocol/src/call.rs`의 CallRequest/CallReply와 `fault.rs::WorkbenchFault/Outcome`를 public caller model로 직접 사용한다. transport lost 뒤 Unknown을 유지하고 same key/payload/instance에 explicit retry를 묶는다. 호스트 helper를 그대로 wrap하는 대안은 replay/outcome/revision 손실 때문에 부적절하다.

## R3 loopback transport와 identity

원 `lifecycle/client.rs::verify_instance_by`, `descriptor.rs::read_descriptor`, `identity.rs::proof` 순서를 참고한다. nonce identify(credential 없음)→HMAC verify→credential handshake(instance/epoch/protocol/storage)→call. 원 HMAC은 SHA256(ownerToken) key, nonce newline instanceId payload다. client-only identity proof는 standard hmac/sha2를 쓰거나 pure shared helper로 추출하며 원 host 비교 vectors를 통과해야 한다. owner secret를 protocol serializer/Debug에 싣지 않는다. descriptor PID는 identity proof가 아니다.

HTTP는 existing lock reqwest0.12.28를 후보로 유지한다. [공식 ClientBuilder](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html)의 no_proxy/redirect/total timeout 및 retry 정책을 확인했고 local installed0.12.28 source에 retry_policy/default retry가 존재한다. 실제 adapter는 `retry(reqwest::retry::never())`, redirect none、no_proxy를 명시한다. exact pinned docs URL fetch 실패는 API 부재 증거가 아니며 설치 source와 compile 검사로 확정한다. `bytes()` whole-body 무제한 allocation 대신 bounded chunks+whole deadline. endpoint IP literal loopback/validated path만 허용, credential 전에 redirect/호환 검사.

## R4 WS와 applied cursor

existing lock tokio-tungstenite/tungstenite0.24.0. [공식 source](https://github.com/snapview/tungstenite-rs/blob/v0.24.0/src/protocol/mod.rs)의 WebSocketConfig max_message_size/max_frame_size는 adapter의 한도 후보. local pinned source·fake oversized fragmented frame regression으로 실제 동작 확인. TS `packages/workbench-client/src/event-client.ts` 및 reconnect/races/gap tests를 source-of-truth transition fixture로 사용하고 UI reducer를 복제하지 않는다. received와applied 분리, consume ACK 이후 cursor, live-first snapshot과 listener/connection operation generation, bounded backlog/close/cancel, hello만으로 gap복구 성공 아님.

## R5 auth/profile/production gate

원 owner authority와 agent scope는 다른 principal이다. 현재 agent capability로 owner HMAC를 검증할 수 있다고 가정하지 않는다. 별도 trusted endpoint/profile issuance 계약은 actual server support가 없으면 gated/unsupported. agent mode는 owner descriptor fallback0. readonly owner lookup은 private file evidence 검사 뒤만 허용한다. generic call은 production gate를 우회하지 못한다. 실제 launch/ensure/stop/배포/production data operations는045/046 readiness 증거 및 operation admission이 필요한 단계로 남는다.

## R6 finite command와 stdin

원 roadmap `docs/client-server-architecture-research.md` CLI/exit contract를 적용한다. status/query 및 catalog 먼저; input/action operation도 same library policy. failure outcome/unknown retry identity는 bounded diagnostic envelope에 보존하며 raw token/private input/details를 그대로 console로 dump하지 않는다. machine warnings는 envelope, log/stdout 분리. 원 `WorkbenchFault.details` 의미는 library에 보존하되 CLI의 diagnostic rendering은 closed safe fields를 별도로 투영한다.

미확정 실제 항목: ES activation/containment/046 migration, agent-profile trusted identity issuance, signed installed executable/update, concurrent actual desktop/TUI matrix. 이를 미해결 없음 또는 readiness PASS로 표현하지 않는다. 이들은 pure model/peer 구현을 막지 않지만 public production activation/전체 feature 완료를 막는 추적된 prerequisite다.


## R7 actual server integration source evidence

사용자 최신 요청으로 merged044 실제 binary/private root wire 경로를 포함한다. core `application/bench_service.rs::open`은 canonical directory→registry.open뿐이고 `application/orchestration/runtime.rs::bootstrap/set_presentation`는 blocking 원 service JSON mutation이다. `service.rs::bootstrap`은 Main-only workspace와 workspace changed를 emit한다. project CRUD는 intent ledger/write를 검증하지만 event 생산을 가정하지 않는다. orchestration bootstrap 후 actual ticket/WS의 Main presentation mutation event를 소비한다. watcher는 `worktree_watcher.rs`에 git Command spawn이 있어 이번 child없는 actual event 경로로 사용하지 않는다. producer/consumer의 exact 입력·stream ownership은 구현 전에 전체 함수/source 및 원 integration fixture로 재확인한다. testserver fixture의 원data root bootstrap은 명시적 private scope이고046 live migration/freeze 우회가 아니다.
