# Contract: Process lifecycle

## 1. 호출 순서

1. domain이 owner와 attempt를 durable reserve한다.
2. caller가 `ProcessSpec`과 예약 identity로 supervisor spawn을 요청한다.
3. platform launcher가 containment를 먼저 만든다.
4. child를 spawn하고 PID/handle/start identity를 containment와 registry에 adopt한다.
5. supervisor가 `#[must_use] UnpublishedProcessLease`를 반환한다. registry는 child ownership을 계속 가진다.
6. caller가 `process_attempt=published`와 해당 run/terminal/operation 상태를 durable commit한다.
7. caller가 `lease.ack_published()`를 호출해 ownership handoff를 끝낸다.
8. accepted reply/started event는 Published durable state를 근거로 내보낸다. ack reply 유실은 같은 attempt로 reconcile한다.

5 이전에는 성공 reply와 accepted/started를 내보내지 않는다. 6 전 실패는 attempt를 Aborted로 바꾸고 child tree를 종료·wait한다. 6 commit 뒤 7 전 caller가 사라지면 process를 즉시 죽이지 않고 durable publication resolver가 Published를 확인해 ownership을 이어받는다.

## 2. cancellation safety

- spawn future가 어느 await 지점에서 drop돼도 registry 또는 platform guard가 partial child를 소유한다.
- lease 반환 뒤 drop/no-ack은 response receiver 상태가 아니라 durable publication state로 판정한다.
- resolver가 `Published`면 registry가 process를 유지하고, `Reserved/Aborted/NotFound`면 stopping으로 간다.
- resolver 일시 실패는 bounded retry 후 fail-closed cleanup+Aborted 보상을 수행하며 무기한 orphan 상태로 두지 않는다.
- raw child handle을 consumer에 넘기지 않는다.
- 같은 owner의 old attempt stop/exit는 new attempt와 매칭되지 않는다.

결정적 cancellation fixture는 (a) adopt response 전, (b) lease 수신 뒤 commit 전, (c) commit 뒤 ack 전, (d) ack 뒤 accepted response 전, (e) resolver storage failure에서 각각 keep/cleanup과 durable state를 검사한다.

## 3. 종료

`graceful tree terminate → policy timeout → force tree kill → direct child/keeper wait → Reaped`

- cancel, timeout, natural exit, server shutdown은 하나의 terminal outcome arbitration을 쓴다.
- 여러 종료 호출은 같은 terminal result를 돌려주는 멱등 호출이다.
- server shutdown complete는 registry live=0, unreaped=0일 때만 가능하다.

## 4. 공개 상태

- `Reserved/Spawning/Adopted`는 내부 상태다.
- 외부 `accepted/started`는 `Published`와 같은 boundary다.
- spawn/adopt 실패에는 started event가 없다.
- 현재 `runner.rs`의 started-before-spawn/adopt 동작을 회귀 시험으로 금지한다.
- 현재 `StartAgentRunUseCase`의 accepted-response-before-launch 동작도 금지한다. HTTP/run start는 adoption/publication까지만 기다리고 agent turn 완료는 기다리지 않는다.

## 5. 진단

허용: owner kind/id, attempt, purpose, lifecycle, timings, exit reason, byte/event counters.

금지: env value, credential, bearer/token, stdin/protocol payload 원문, 전체 argv 중 secret로 표시된 값.
