# Quickstart: ProcessSupervisor 검증

## 전제

- feature 구현 branch의 clean tree
- Rust workspace toolchain
- 해당 target에서 process inspection 권한
- production inventory 정본: [contracts/process-inventory.md](./contracts/process-inventory.md)

## 1. 정적 inventory

production source scan을 실행한다.

기대 결과:
- ServerOwned 직접 spawn 0
- 분류 누락 0
- daemon/native/build/fixture/other-app가 별도 합계로 출력됨

## 2. lifecycle ordering

spawn success, missing executable, adopt failure, rapid exit와 다음 cancellation fixture를 실행한다: adopt response 전, lease 수신 뒤 durable commit 전, publication CAS/cleanup CAS 각각의 winner, commit 뒤 ack 전, ack 뒤 accepted response 전, publication resolver storage failure/ambiguous commit, outbox commit/send/ack 전후 crash.

기대 결과:
- 실패/cancel에서 started 0, live child 0
- 성공에서 Adopted 뒤 started 정확히 1
- commit 뒤 ack 유실은 같은 attempt를 유지하고, commit 전 유실은 cleanup
- publication과 cleanup 중 CAS winner 하나만 process 수명과 logical event를 결정
- reconnect/replay 뒤에도 attempt별 사용자 projection 적용 1회
- run start HTTP/호출 response는 adoption/publication 뒤지만 전체 agent turn 종료 전
- old attempt late completion이 new attempt를 바꾸지 않음

## 3. output contract

protocol exact-limit, limit+1, malformed, mid-frame EOF, incomplete-frame slow-loris, endless valid frames와 display stdout/stderr 100 MiB/no-newline fixture를 실행한다.

기대 결과:
- protocol 손상은 typed fatal failure이며 이후 frame 성공 없음
- display log는 bounded counters/marker를 남김
- status/cancel은 2초 안에 응답
- credential sentinel 0

## 4. tree containment

direct child, grandchild, signal-ignore, env-clear+exec, leader exit, session/group escape, double-fork fixture에 cancel, timeout, server graceful stop, keeper-only hard kill, server+keeper hard kill을 적용한다.

기대 결과:
- descendant 0
- unreaped direct child 0
- PID reuse 대조 process 생존
- durable/transient domain owner 모두 recovery anchor로 reconcile
- live server가 keeper death를 즉시 감지하고 cleanup ownership 인계

## 5. consumer regression

- ACP long turn/queued turn/permission/cancel
- terminal create/output/wait/kill
- Git history/detail/diff/status/worktree operations
- watcher refresh와 orchestration worktree guard
- catalog online/fallback cache와 login-shell timeout
- AW server wait-stop/force-stop/process suites

각 suite는 filtered 0이 아닌 실제 test 수와 exit code를 기록한다.

## 6. platform matrix

macOS, Linux, Windows의 실제 CI job에서 [platform-containment.md](./contracts/platform-containment.md) 표를 채운다. 한 target의 결과를 다른 target 근거로 쓰지 않는다.

production consumer migration 전에 격리된 feasibility spike로 target별 API/권한, nonce 제거 descendant 추적, 재사용 안전 signal handle, keeper death ownership을 입증한다. 입증하지 못한 target은 미완료 blocker로 남기며 group-kill 통과나 다른 target 결과로 대체하지 않는다.

## 7. 전체 gate

workspace fmt, strict Clippy, Rust all-targets, TypeScript check, frontend tests/build, workbench-core integration, AW integration을 실행하고 단계별 exit code를 항상 남긴다.
