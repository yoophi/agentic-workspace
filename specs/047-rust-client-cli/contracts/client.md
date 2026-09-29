# Connection/call/event 계약

## Connection

IP literal127.0.0.1 또는[::1] loopback만; credentials/userinfo/query/fragment가 든 base URL 및 redirect/proxy 거절. owner-only bounded descriptor no-follow FD open→uid/mode/file type/size 확인→read, read source 교체는 open한 FD identity 기준. PID 생존으로 endpoint trust를 만들지 않는다. TCP connection을 직접 소유한 HTTP sender에서 fresh nonce identify MAC 확인 전 Authorization0, identify→credential handshake/call은 같은 TCP connection sender로 보내고 server close/reconnect/pool replacement가 있으면 credential 없는 fresh proof부터 다시 확인한다. authenticated retry/implicit reconnect는 transport에서 금지한다. 별도WS TCP도 ticket upgrade 전 같은 socket에서 identify proof를 확인하고 upgrade로 넘긴다. 고정URL/이전 proof만으로 새 연결에 credential을 보내지 않는다. handshake epoch는 nonempty/descriptor 일치와 supported protocol/storage. missing server는 unavailable; ensure/spawn 호출 없음.

## Call

전체 CallRequest/CallReply/WorkbenchFault DTO 및 catalog를 재사용한다. bounded body+whole deadline, HTTP/body status·requestId·kind shape 검증, invalid reply는 성공/default null로 만들지 않음. transport loss Unknown, server 명시 NotApplied/Fault와 구분. automatic HTTP retry0. unknown attempt를 가진 retry는 exact key+immutable input+instance+epoch+operationGeneration, newepoch이면 old attempt를 unresolved로 유지하고 자동 재제출0. 401 refresh 후에도 instance/epoch rule 유지. local wait cancel/timeout은 원 작업의 implicit cancel0.

원 command key namespace의 Durable/Epoch 의미를 catalog로 구분한다. 더 오래 살 수 있는 Durable key도 client가 epoch 변경 뒤 arbitrary resubmit하여 효과를 추정하지 않는다. capability acquisition이 없는 agent operation은 Unsupported/PrerequisiteUnavailable로 거절하며 owner로 downgrade하지 않는다. generic call도 동일 production activation gate를 통과해야 한다.

## Event

POST ticket→WS(credential URL 금지; ticket secret은 redact)→hello identity/cursor 확인→event consume ACK. 최대message/frame1MiB、queue256/8MiB、recovery attempts5, backoff250ms..10s jitter. protocol 초과는 typed error+close, arbitrary truncate/drop continue 금지. non-retaining worktree signal의 reconnect snapshot과 retained run/orchestration/exchange 의미를 TS source fixtures에 대조한다. gap live 확보→hello→snapshot→consumer reset→snapshot기준 buffer filtering, hello만으로 성공 아님. all consumer applied min cursor, consumer 실패는 해당 generation reset, unregister/old promise/newepoch guards 양방향 시험. cancellation은 모든 task/socket JoinHandle settle bounded 확인.

## Public activation

fake peer는 test-only VerifiedEndpoint와 admission fixture를 쓴다. production에서 factory bool/testenv로 readiness를 우회할 수 없음. 현재 wire에045/046 readiness proof가 없으므로 controlled caller와 owner readonly status/query 및 source 확인된 비실행 closed operation만 독립 후보이고 process 실행/data migration/ensure/stop/agent profile activation은 명시 prerequisite unavailable. server enforcement를 구현했다고 가정하지 않고 이후 연결 task로 추적한다. task 완료/ship은 scoped gate 완료에 따라 기록한다.


## Actual merged044 wire conformance

baseline actualbinary20fcd5f + private root fixture로 identity/handshake、system.describe/project.list、project.create/update/delete/replay와 event route를 검증한다. project event는 존재한다고 가정하지 않는다. 실제bench.open→orchestration.bootstrap(Main only)→orchestration stream ticket/WS hello 및 bootstrap event ACK→empty-workspace recover의 runtimeReconciled event와 snapshot revision r+1을 사용한다. recover input은 {benchId}, 원 required scope orchestration:write, stream orchestration:<benchId>, schema orchestration.workspaceUpdated.v1이다. empty fixture precondition은 Main1/currentRunId null 및 generations/tasks/reports/commands/coordinatorNotifications/promptDispatches0, active generation null, in-flight0다. 외부/다른 caller 없는 private fixture에서만 test-only trigger authority를 쓰며 production generic recover는 gate를 유지한다. CLI는 project CRUD 실제 mutation 및 actual event consumption을 검증한다. arbitrary workspace recover를 안전한 read/mutation으로 공개하지 않는다. worker/agent launch 없음은 원 service/runtime 호출과 run/bench 상태/fixture process evidence로 확인한다. 명시적 test server process의 startup deadline/kill+bounded wait/reap가 실패하면 assertion 실패와 cleanup failure를 모두 남긴다. 서버status/stop fixture cleanup은 harness authority로 실행하며 CLI public stop readiness와 별개다. fakepeer race 통과만으로 wire완료 아님.


## CLI invocation을 넘는 retry identity

mutation은 첫 submission 전에 owner-only private retry state를 저장한다: version、operation、protocol/contract、request/key、immutable input bytes/digest、instance/epoch、attempt generation、state/outcome. original input은 console에 출력하지 않는다. state는 data backup 대상 서버store가 아닌 caller runtime-control domain이며 root/path/no-follow/owner/mode/size와 atomic fsync update를 검증한다. submit 전 저장이 실패하면 요청0. unknown 상태에서 다음 invocation은 `--retry-state FILE`로 원 요청만 읽고 key/input flag 변경을 거절한다. verified instance/epoch 불일치는 unresolved로 유지하고 HTTP command0. arbitrary `--idempotency-key`만으로 이전 unknown 요청을 검증했다고 주장하지 않는다. explicit 새 operation은 별도 state/key이며 이전 unknown의 성공/실패 추정 없음. crash commit-before-output、state write 실패、old invocation completion/CAS、caller cancellation、reopen retry fixtures를 tasks에 포함한다. CLI private state가 있음에도 production all-writer/freeze/restore proof로 계산하지 않는다.
