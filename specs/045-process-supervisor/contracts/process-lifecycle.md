# Contract: Process lifecycle

## 1. 호출 순서

1. durable business execution은 domain owner/attempt를 먼저 예약한다. read-only helper는 authenticated request/server owner만 만들고 domain 의미는 transient로 유지한다.
2. 모든 ServerOwned child는 supervisor store에 durable containment recovery anchor를 `Reserved`로 기록한다. 이는 transient helper를 재실행 가능한 business operation으로 만들지 않는다.
3. caller가 `ProcessSpec`과 owner/attempt identity로 supervisor spawn을 요청한다.
4. platform launcher가 containment를 먼저 만든다.
5. child를 spawn하고 PID/handle/start identity를 containment와 registry에 adopt한 뒤 anchor를 `Adopted`로 바꾼다.
6. supervisor가 `#[must_use] UnpublishedProcessLease`를 반환한다. registry는 child ownership을 계속 가진다.
7. 외부 accepted/started가 있는 caller는 하나의 SQLite transaction에서 `Adopted → Published` CAS, domain 상태/재시도 response, attempt-keyed outbox를 commit한다. transient helper는 외부 publication 없이 `Adopted → Active` CAS한다.
8. caller가 `lease.ack_published()` 또는 `lease.ack_active()`를 호출해 ownership handoff를 끝낸다.
9. accepted reply/started event는 committed response/outbox를 전달한다. ack/reply 유실은 같은 attempt와 event id로 reconcile한다.

6 이전에는 성공 reply와 accepted/started를 내보내지 않는다. publication/activation 전 실패는 abort CAS를 이긴 뒤 child tree를 종료·wait한다. commit 뒤 ack 전 caller가 사라지면 process를 즉시 죽이지 않고 durable resolver가 winner state를 확인해 ownership을 이어받는다.

## 1.1 publication과 cleanup의 단일 승자

- `Adopted`에서 허용되는 첫 durable 전이는 `Published`, `Active`, `Aborting` 중 하나다.
- publication transaction은 `WHERE state = 'adopted'` CAS로 `Published`를 만들며 domain result와 outbox를 같은 transaction에 쓴다.
- resolver cleanup은 같은 행에 `Adopted → Aborting` CAS를 수행한 뒤에만 signal/wait를 시작한다.
- publication/activation이 이기면 resolver는 keep하고, `Aborting`이 이기면 caller는 typed `PublicationLost`를 받아 reply/event를 내보내지 않는다.
- retry는 이미 정해진 winner와 같은 결과를 반환한다. `Published`를 `Aborting`으로, `Aborting`을 `Published`로 뒤집지 않는다.
- storage 결과가 ambiguous하거나 읽기 불능이면 cleanup을 추정하지 않는다. child는 containment 안에서 quarantined 상태로 유지하고 readiness를 내린 채 reconcile을 계속한다. server shutdown은 platform containment로 정리하되, 저장소 복구 전 공개 성공을 만들지 않는다.

## 2. cancellation safety

- spawn future가 어느 await 지점에서 drop돼도 registry 또는 platform guard가 partial child를 소유한다.
- lease 반환 뒤 drop/no-ack은 response receiver 상태가 아니라 durable publication state로 판정한다.
- resolver가 `Published/Active`면 registry가 process를 유지한다. `Adopted`에서 `Aborting` CAS를 이긴 경우와 이미 `Aborting/Aborted`인 경우만 stopping으로 간다.
- `Reserved/NotFound`는 spawn/adopt가 완료됐다는 in-memory 증거와 모순이므로 임의 cleanup하지 않고 corruption recovery 경로로 격리한다.
- resolver storage failure는 bounded backoff로 재시도하고 server readiness를 내린다. 확인 없이 published child를 죽이거나 unpublished child를 공개하지 않는다.
- raw child handle을 consumer에 넘기지 않는다.
- 같은 owner의 old attempt stop/exit는 new attempt와 매칭되지 않는다.

결정적 cancellation fixture는 (a) adopt response 전, (b) lease 수신 뒤 commit 전, (c) publication CAS와 abort CAS가 각각 이기는 두 barrier interleaving, (d) commit 뒤 ack 전, (e) ack 뒤 accepted response 전, (f) resolver storage failure/ambiguous commit, (g) outbox commit 전후 crash에서 각각 keep/cleanup, durable state, logical event 1개를 검사한다.

## 3. 종료

`graceful tree terminate → policy timeout → force tree kill → direct child/keeper wait → Reaped`

- cancel, timeout, natural exit, server shutdown은 하나의 terminal outcome arbitration을 쓴다.
- 여러 종료 호출은 같은 terminal result를 돌려주는 멱등 호출이다.
- server shutdown complete는 registry live=0, unreaped=0일 때만 가능하다.

## 4. 공개 상태

- `Reserved/Spawning/Adopted/Aborting`은 내부 상태다.
- 외부 `accepted/started`는 `Published`와 같은 boundary다.
- `Active`는 외부 publication이 없는 transient helper의 실행 상태다.
- spawn/adopt 실패에는 started event가 없다.
- 현재 `runner.rs`의 started-before-spawn/adopt 동작을 회귀 시험으로 금지한다.
- 현재 `StartAgentRunUseCase`의 accepted-response-before-launch 동작도 금지한다. HTTP/run start는 adoption/publication까지만 기다리고 agent turn 완료는 기다리지 않는다.

## 4.1 durable logical publication

- `Published` transaction은 `(attempt_id, event_kind)` unique key의 outbox와 idempotent HTTP/command result를 함께 기록한다.
- dispatcher는 outbox event id를 WS sequence/replay에 연결하고 delivery acknowledgement 전까지 재전달할 수 있다.
- transport 중복은 같은 event id/attempt를 기준으로 client projection이 무시한다. 관측 가능한 accepted/started는 attempt마다 durable logical event 한 개다.
- crash가 transaction 전이면 event 0개와 cleanup, transaction 뒤면 recovery/replay를 포함해 logical event 1개다.

## 5. 진단

허용: owner kind/id, attempt, purpose, lifecycle, timings, exit reason, byte/event counters.

금지: env value, credential, bearer/token, stdin/protocol payload 원문, 전체 argv 중 secret로 표시된 값.
