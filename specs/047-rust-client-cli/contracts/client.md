# Connection/call/event 계약

## Connection

IP literal127.0.0.1 또는[::1] loopback만; credentials/userinfo/query/fragment가 든 base URL 및 redirect/proxy 거절. owner-only bounded descriptor no-follow FD open→uid/mode/file type/size 확인→read, read source 교체는 open한 FD identity 기준. PID 생존으로 endpoint trust를 만들지 않는다. TCP connection을 직접 소유한 HTTP sender에서 fresh nonce identify MAC 확인 전 Authorization0, identify→credential handshake/call은 같은 TCP connection sender로 보내고 server close/reconnect/pool replacement가 있으면 credential 없는 fresh proof부터 다시 확인한다. authenticated retry/implicit reconnect는 transport에서 금지한다. 별도WS TCP도 ticket upgrade 전 같은 socket에서 identify proof를 확인하고 upgrade로 넘긴다. 고정URL/이전 proof만으로 새 연결에 credential을 보내지 않는다. handshake epoch는 nonempty/descriptor 일치와 supported protocol/storage. missing server는 unavailable; ensure/spawn 호출 없음.

## Call

전체 CallRequest/CallReply/WorkbenchFault DTO 및 catalog를 재사용한다. bounded body+whole deadline, HTTP/body status·requestId·kind shape 검증, invalid reply는 성공/default null로 만들지 않음. transport loss Unknown, server 명시 NotApplied/Fault와 구분. automatic HTTP retry0. unknown attempt를 가진 retry는 exact key+immutable input+instance+epoch+operationGeneration, newepoch이면 old attempt를 unresolved로 유지하고 자동 재제출0. 401 refresh 후에도 instance/epoch rule 유지. local wait cancel/timeout은 원 작업의 implicit cancel0.

원 command key namespace의 Durable/Epoch 의미를 catalog로 구분한다. 더 오래 살 수 있는 Durable key도 client가 epoch 변경 뒤 arbitrary resubmit하여 효과를 추정하지 않는다. capability acquisition이 없는 agent operation은 Unsupported/PrerequisiteUnavailable로 거절하며 owner로 downgrade하지 않는다. generic call도 동일 production activation gate를 통과해야 한다.

## Event

POST ticket→WS(credential URL 금지; ticket secret은 redact)→hello identity/cursor 확인→event consume ACK. 최대message/frame1MiB、queue256/8MiB、recovery attempts5, backoff250ms..10s jitter. protocol 초과는 typed error+close, arbitrary truncate/drop continue 금지. non-retaining notification의 disconnect 및 같은 epoch SubscriberLagged/Shutdown은 live-only 재구독 후 hello→fresh snapshot→모든 consumer reset 완료 뒤 delivery한다. 과거 notification replay를 가정하지 않는다. retained run/orchestration/exchange는 applied cursor replay와 독립 consumer reset을 유지하며 TS source fixtures에 대조한다. gap live 확보→hello→snapshot→consumer reset→snapshot기준 buffer filtering, hello만으로 성공 아님. all consumer applied min cursor, consumer 실패는 해당 generation reset, unregister/old promise/newepoch guards 양방향 시험. cancellation은 모든 task/socket JoinHandle settle bounded 확인.

## Public activation

fake peer는 test-only VerifiedEndpoint와 admission fixture를 쓴다. production에서 factory bool/testenv로 readiness를 우회할 수 없음. 현재 wire에045/046 readiness proof가 없으므로 controlled caller와 owner readonly status/query 및 source 확인된 비실행 closed operation만 독립 후보이고 process 실행/data migration/ensure/stop/agent profile activation은 명시 prerequisite unavailable. server enforcement를 구현했다고 가정하지 않고 이후 연결 task로 추적한다. task 완료/ship은 scoped gate 완료에 따라 기록한다.


## Actual merged044 wire conformance

baseline actualbinary20fcd5f + private root fixture로 identity/handshake、system.describe/project.list、project.create/update/delete/replay와 event route를 검증한다. project event는 존재한다고 가정하지 않는다. 실제bench.open→orchestration.bootstrap(Main only)→orchestration stream ticket/WS hello 및 bootstrap event ACK→empty-workspace recover의 runtimeReconciled event와 snapshot revision r+1을 사용한다. recover input은 {benchId}, 원 required scope orchestration:write, stream은 bootstrap 응답의 non-null `eventStreamId`(`orchestration:<bindingId>`)를 그대로 사용하며 schema는 orchestration.workspaceUpdated.v1이다. bindingId는 서버가 독립적으로 생성하므로 benchId 또는 workspaceId로 streamId를 조립하지 않는다. ticket cursor와 WS hello, 수신 envelope, applied cursor는 모두 반환된 eventStreamId에 묶는다. empty fixture precondition은 Main1/currentRunId null 및 generations/tasks/reports/commands/coordinatorNotifications/promptDispatches0, active generation null, in-flight0다. 외부/다른 caller 없는 private fixture에서만 test-only trigger authority를 쓰며 production generic recover는 gate를 유지한다. CLI는 project CRUD 실제 mutation 및 actual event consumption을 검증한다. arbitrary workspace recover를 안전한 read/mutation으로 공개하지 않는다. worker/agent launch 없음은 원 service/runtime 호출과 run/bench 상태/fixture process evidence로 확인한다. 명시적 test server process의 startup deadline/kill+bounded wait/reap가 실패하면 assertion 실패와 cleanup failure를 모두 남긴다. 서버status/stop fixture cleanup은 harness authority로 실행하며 CLI public stop readiness와 별개다. fakepeer race 통과만으로 wire완료 아님.


## CLI invocation을 넘는 retry identity

mutation은 첫 submission 전에 owner-only private retry state를 저장한다: version、operation、protocol/contract、request/key、immutable input bytes/digest、instance/epoch、attempt generation、state/outcome. original input은 console에 출력하지 않는다. state는 data backup 대상 서버store가 아닌 caller runtime-control domain이며 root/path/no-follow/owner/mode/size와 atomic fsync update를 검증한다. submit 전 저장이 실패하면 요청0. unknown 상태에서 다음 invocation은 `--retry-state FILE`로 원 요청만 읽고 key/input flag 변경을 거절한다. verified instance/epoch 불일치는 unresolved로 유지하고 HTTP command0. arbitrary `--idempotency-key`만으로 이전 unknown 요청을 검증했다고 주장하지 않는다. explicit 새 operation은 별도 state/key이며 이전 unknown의 성공/실패 추정 없음. crash commit-before-output、state write 실패、old invocation completion/CAS、caller cancellation、reopen retry fixtures를 tasks에 포함한다. CLI private state가 있음에도 production all-writer/freeze/restore proof로 계산하지 않는다.

## 실제 이벤트의 결정적 acceptance (D-C2)

저장소 revision과 EventEnvelope sequence를 혼동하지 않는다. 원 persist_mutation은 commit 후 revision을 r+1로 발행한다. 뒤의 notification pass는 get_for_bench snapshot을 읽고 emit_runtime_update_for→emit_runtime_update→DeliveryOrchestrationSink.emit→EventHub.publish_state로 같은 revision을 다시 발행한다. helper/sink는 저장소 revision을 증가시키지 않고 hub만 같은 stream lock에서 sequence를1 증가시킨다. 빈 fixture의 다른 writer/notification0 조건에서 다음을 각각 exact assertion으로 검증한다.

1. bootstrap event를 실제 소비·출력 ACK한 기준 (workspaceId, streamId, epoch, revision=r, sequence=s)을 저장한다. ticket/hello만으로 bootstrap ACK를 대신하지 않는다.
2. empty recover를1회 제출하고 `schema=orchestration.workspaceUpdated.v1`, 동일 workspace/stream/epoch, `reason=runtimeReconciled`, `revision=r+1`, `sequence=s+1`인 target envelope를 반드시 실제 수신·소비·ACK한다. 다른 reason의 이벤트는 target 완료로 세지 않는다.
3. 뒤의 `reason=notificationRecovery`, 동일 identity/schema, `revision=r+1`, `sequence=s+2` envelope도 별도 소비·ACK한다. 같은 revision이라는 이유로 이 이벤트를 중복 제거하지 않는다. cursor는 완전한 JSONL 출력/consumer ACK 뒤 s→s+1→s+2로만 전진한다. 예상 외 envelope/중복/누락은 실패이며 `>=`로 숨기지 않는다.
4. HTTP recover reply와 WS event 도착 사이에는 총순서를 가정하지 않는다. 두 envelope ACK와 reply 성공을 명시적 완료 신호로 기다린 뒤 별도 orchestration.get으로 최종 snapshot을 읽고 revision=r+1 및 empty/run0 상태를 exact 비교한다. recover reply의 snapshot도 r+1이어야 한다. 유한 deadline은 실패 판정용이며 임의 sleep/timeout만으로 두 이벤트 완료를 추정하지 않는다.
5. controlled peer에서는 reply-before-events와 두 events-before-reply 양쪽을 barrier로 강제하여 ACK/완료 reducer를 시험한다. 실제 서버에서는 관측된 순서와 두 eventId/sequence/reason/revision·reply·최종 snapshot을 보존하며 원 product 코드를 시험에 맞춰 변경하지 않는다. fixture teardown은 두 이벤트 처리 뒤 수행하며 누락/cleanup 실패는 별도로 기록한다.

이 acceptance는 T032/T033 controlled 양방향 barrier와 T038 exact merged044 binary/실제 aw subprocess에서 검증했다. 실제 명령·artifact SHA·event vector·cleanup 증거는 validation.md의 T034–T039 checkpoint에 기록했다. 최종 소스 재검증·순차 구현 리뷰·PR/CI/merge 및 인계 완료는 별도 최종 gate다.

## 스트림 식별자와 재바인딩 (D-C3)

`benchId`는 recover/get 입력과 작업대 권한 확인용이고, `workspaceId`는 저장된 작업 영역 식별자다. `bindingId`는 현재 작업 영역과 작업대의 묶임마다 발급된다. bootstrap 응답의 `eventStreamId`가 구독 주소이며 missing/null이면 명시적으로 실패한다. 동일 workspace를 다른 bench에 재바인딩하면 새 eventStreamId를 취득하고 새 consumer generation을 연다. 이전 스트림의 evicted gap 또는 늦은 ACK를 새 스트림 cursor로 이식하지 않는다. 실제 통합 fixture는 응답 eventStreamId와 ticket/hello/envelope identity의 일치를 확인하고, controlled fixture는 독립적인 bench/workspace/binding ID와 재바인딩을 사용해 ID 혼동을 검출한다. 기존 서버 동작을 바꾸어 잘못된 client 식별자를 허용하지 않는다.

retry state의 저장/읽기 상한은 HTTP raw body와 독립적이다(기본256MiB). submission 전에 현재 request/identity serialized bytes +24×최대 raw body +64KiB를 예약한다. 숫자/문자열의 parse→serialize 정규화를 포함하며 공간 부족/overflow는 전송 전에 오류다. finite JSON 숫자24bytes 이하와 문자열 escape6배 이하의 허용 형식을 전제로 한다. 완료 응답은 원 identity/outcome과 함께 저장되어 동일 요청 cache reopen이 추가 HTTP resubmit 없이 가능하다.

snapshot completion은 owner/generation/scope를 오류 분류보다 먼저 검증한다. old terminal 오류도 stale이며 새 Live에 영향0이다. 현재 Identity/Incompatible/Protocol/auth 및 nonretryable fault는 원 cause를 보존하여 terminal 정리하며 추가 snapshot0이다. transient allowlist는 connect와 동일하고 applied/boundary cursor 및 bounded budget/backoff를 유지한다. Evicted terminal 및 listener의 첫 snapshot은 즉시, 이후 snapshot/reset 재시도는 동일 exponential/jitter backoff 뒤 실행한다. delay는 소유 snapshot task 안에 있어 actor의 새 gap/종료를 막지 않으며 abort+join으로 취소한다. request timeout은 delay 이후 시작한다. live-first stream은 기존 Connect backoff를 사용하고 snapshot에서 중복 대기하지 않는다.

## 직렬화 예산 비교 (I-C5)

wire 상한은 변경하지 않는다. Limits는 아래 표현의 상한을 각각 검증하며 부족/overflow는 typed failure다. cursor는 stream/epoch/keys와 afterSequence=u64::MAX의 직렬화 크기를 input1MiB에 포함하여 future sequence 증가분도 예약한다. construction/rebind/snapshot 및 JSONL open/reset/end에서 검사한다.

| 표현 | 기본 상한 | 포함 범위 |
|---|---|---|
| raw HTTP body |8MiB|실제 수신 bytes, 정규화 전|
| raw WS frame/message |1MiB 각각|실제 frame/message bytes|
| normalized snapshot value |192MiB|serde Value 전체 직렬화, raw Body 최대24배|
| serialized event queue |전체8MiB/256items|EventEnvelope를 포함한 aggregate reader/channel/inflight/reducer event budget|
| JSONL event |record256MiB 이내|queue가 허용한 serialized envelope + type/event wrapper +newline|
| JSONL reset |record256MiB 이내|snapshot value + cursor/applied 각 최대1MiB +wrapper+newline|
| JSONL open/end |record256MiB 이내|cursor +open/end wrapper, end safe error projection +newline|

JSONL config는 `max(snapshot_bytes, queue_bytes) +2*input_bytes +128KiB` 이상의 크기와 overflow 부재를 요구한다.128KiB는 bounded wrapper/error projection/newline 여유이며 cursor는 별도 input 표현 상한이다. write는 최종 직렬화 bytes에 newline을 붙인 뒤 JsonlRecord를 검사하고 write_all/flush 완료만 ACK한다. raw exponent 정규화는 snapshot/retry-state 예약의24배에 포함하며 record가 raw Body와 같은 길이라는 가정은 없다. 이 값은 즉시 확보하는 버퍼가 아닌 검증 상한이다.
