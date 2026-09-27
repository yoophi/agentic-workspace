# 변경 operation 네트워크 공개 증거 (042 research R13·R17, T018)

`ExposurePolicy::network_default()`(전체 공개)의 근거. 정상 경로 parity(계약 suite)와 descriptor 분류는 증거로 쓰지 않는다. 각 operation은 아래 세 질문에 시험으로 답한다.

- **중단**: 효과 도중 프로세스가 죽으면 재시작 판정이 맞는가(자동 재실행 없음).
- **재시작**: 재시작 뒤 같은 키 재시도가 효과를 다시 내지 않는가.
- **단절**: HTTP 연결이 효과 뒤·결과 기록 전에 끊기고 같은 서버에 같은 키로 재시도하면 효과가 1회인가.

## 공유 실행 경로

| 경로 | 코드 | 적용 operation | 성질 |
|---|---|---|---|
| **L** 영속 ledger(intent-first) | `workbench-core/src/application/intent_first.rs` `IntentFirstRunner::run` → `execute`(`spawn_blocking`) | 14개 | ledger pending → 저장 → applied. 중단 지점 hook(`crash_if`·`pause_at`)이 이 함수 하나에 있다. `spawn_blocking`은 시작 뒤 취소되지 않는다 |
| **E** 세대 멱등 | `workbench-core/src/application/handlers/epoch.rs` `EpochHandler::handle` → `epoch_idempotency.rs` `EpochIdempotency::run` | 39개 | 결과는 성공 뒤 메모리에 기록, 세대와 함께 사라진다. 범위: `Bench`·`Open`·`RunOwner`·`None` |
| **H** HTTP 호출 | `workbench-server/src/routes/calls.rs` 분리 `tokio::spawn` + `drain.rs` | 전부 | 받아들인 호출은 연결과 무관하게 끝까지, 종료는 drain 뒤 |
| **O** orchestration 파일 | `workbench-core/src/infrastructure/fs/legacy_json_store.rs` `atomic_write`(temp → rename, `.bak`) | orchestration 변경 26개(E 위) | 끊긴 쓰기는 이전 판 유지 |

## 대표 시험

| 표지 | 시험 | 무엇을 증명 |
|---|---|---|
| C-P | `ledger_crash_points.rs` (037) | project.create 세 지점 판정, 같은 키 재생·unknown 차단 |
| C-S | `us1_crash_points.rs` (038) | savedPrompt.create/delete, goal.create/recordProgress, agentRunSettings.save 세 지점 |
| C-U | `crash_points_updates.rs` (042 T010) | project.update/delete, savedPrompt.update, goal.update/clear × 세 지점 |
| C-G | `git_reconcile.rs` (038) | git create/deleteWorktree pending 판정(경로 관찰) |
| C-R | `run_start_reconcile.rs` (040) | run.start 완료 재생·중단 unknown |
| C-O | `legacy_json_store.rs` 단위 `interrupted_writes_leave_the_previous_version_readable` (042) | orchestration 파일 끊긴 쓰기 → 이전 판, 깨진 본 파일 → `.bak` |
| E-I | `epoch_idempotency.rs` (040) | 동시 같은 키 1회 실행, 기록 넘침 뒤 재시도 비실행, 닫힌 작업대 `notFound` |
| R-E | `restart_retry.rs` `epoch_scoped_commands_are_not_found_after_restart` (042 T011) | 재시작 뒤 bench.close(`closed:false`)·run.sendPrompt·exchange.send → 효과 없음, HTTP 동일 |
| R-O | `restart_retry.rs` `orchestration_commands_apply_once_across_restart` | bootstrap·bindCoordinator·delegateGoal 재시도 → `notFound`, 파일 쓰기·prompt 없음 |
| R-A | `restart_retry.rs` `agent_orchestration_commands_are_rejected_after_restart_without_effect` | agent 변경(RunOwner) 재시도 → 역할 없음 거절, 파일·자식 기동 없음 |
| R-B | `restart_retry.rs` `bench_open_retry_after_restart_opens_a_new_bench` | bench.open → 새 작업대(설계 리뷰 D2), 이전 id `notFound` |
| D-L | `http_disconnect_retry.rs` `disconnected_ledger_write_retry_applies_once` | L 경로(project.create): 저장 뒤·ledger 확정 전 단절 → 1회 |
| D-R | `…disconnected_run_start_retry_starts_once` | run.start: 기동 뒤 단절 → 1회 |
| D-E | `…disconnected_prompt_retry_applies_once` | E·Bench 범위(run.sendPrompt): 효과 뒤 단절 → 1회 |
| D-O | `…disconnected_orchestration_write_retry_applies_once` | E·O(delegateGoal): 파일 revision +1, prompt 1회 |
| D-A | `…disconnected_agent_child_creation_retry_starts_one_child` | E·RunOwner(createChildTask): 자식 1회, 저장 결과가 원 요청의 자식 |
| D-S | `…shutdown_drains_a_disconnected_call_before_returning`, `…calls_after_the_shutdown_signal_are_rejected_without_effect` | 단절 → 종료 신호 → drain 뒤 반환, 종료 뒤 새 호출 503 |

변이(같은 진입점, `tasks.md` 실행 기록): H의 분리 실행 제거 → D-E·D-O·D-A **효과 2회**, D-S 조기 반환으로 실패. D-L·D-R은 통과 — L 경로는 `spawn_blocking` 비취소와 ledger가 막으므로 H와 무관하게 안전하다(이 두 시험은 L 경로 자체의 단절 안전 증거).

## operation별

범위: E 경로의 멱등 기록 범위. 대표 시험이 같은 공유 경로를 쓰면 그 시험을 적는다(같은 코드 경로·같은 범위일 때만).

| # | operation | 종류 | 범위/경로 | 중단 | 재시작 | 단절 |
|---|---|---|---|---|---|---|
| 1 | project.create | L | L | C-P | C-P(재생) | D-L |
| 2 | project.update | L | L | C-U | C-U | D-L |
| 3 | project.delete | L | L | C-U | C-U | D-L |
| 4 | savedPrompt.create | L | L | C-S | C-S | D-L |
| 5 | savedPrompt.update | L | L | C-U | C-U | D-L |
| 6 | savedPrompt.delete | L | L | C-S | C-S | D-L |
| 7 | goal.create | L | L | C-S | C-S | D-L |
| 8 | goal.update | L | L | C-U | C-U | D-L |
| 9 | goal.clear | L | L | C-U | C-U | D-L |
| 10 | goal.recordProgress | L | L | C-S | C-S | D-L |
| 11 | agentRunSettings.save | L | L | C-S | C-S | D-L |
| 12 | git.createWorktree | L | L(부작용 git) | C-G | C-G | D-L(같은 `execute`, 부작용 뒤 `pause_at`) |
| 13 | git.deleteWorktree | L | L(부작용 git) | C-G | C-G | D-L |
| 14 | run.start | L | L + 엔진 기동 | C-R | C-R | D-R |
| 15 | bench.open | E | Open | 메모리(세대와 소멸) | R-B | E-I·H |
| 16 | bench.close | E | None(종료 상태 멱등) | 메모리 | R-E | 자연 멱등(재실행해도 `closed:false`) |
| 17 | bench.requestTitle | E | RunOwner | 메모리 | R-A(같은 범위·run 없음) | D-A(같은 범위) |
| 18 | run.sendPrompt | E | Bench | 메모리 | R-E | D-E |
| 19 | run.steer | E | Bench | 메모리 | R-E(작업대 조회 `notFound`) | D-E(같은 범위) |
| 20 | run.cancelAndSend | E | Bench | 메모리 | R-E | D-E |
| 21 | run.setPermissionMode | E | Bench | 메모리 | R-E | D-E |
| 22 | run.cancel | E | Bench | 메모리 | R-E | D-E |
| 23 | run.respondPermission | E | Bench | 메모리 | R-E | D-E |
| 24 | exchange.syncWorkspace | E | Bench | 메모리 | R-E | D-E |
| 25 | exchange.send | E | Bench | 메모리 | R-E | D-E |
| 26 | exchange.acknowledge | E | Bench | 메모리 | R-E | D-E |
| 27 | exchange.sendFromRun | E | RunOwner | 메모리 | R-A | D-A |
| 28 | orchestration.bootstrap | E | Bench + O | C-O | R-O | D-O(같은 범위·O) |
| 29 | orchestration.bindCoordinator | E | Bench + O | C-O | R-O | D-O |
| 30 | orchestration.delegateGoal | E | Bench + O + prompt | C-O | R-O | D-O |
| 31 | orchestration.adoptManualChild | E | Bench + O | C-O | R-O | D-O |
| 32 | orchestration.setPresentation | E | Bench + O | C-O | R-O | D-O |
| 33 | orchestration.sendChildCommand | E | Bench + O + prompt | C-O | R-O | D-O |
| 34 | orchestration.respondInput | E | Bench + O + prompt | C-O | R-O | D-O |
| 35 | orchestration.cancelTask | E | Bench + O + 엔진 | C-O | R-O | D-O |
| 36 | orchestration.retryTask | E | Bench + O + 기동 | C-O | R-O | D-O·D-A(기동 1회) |
| 37 | orchestration.reassignTask | E | Bench + O + 기동 | C-O | R-O | D-O·D-A |
| 38 | orchestration.handoffCoordinator | E | Bench + O | C-O | R-O | D-O |
| 39 | orchestration.dispatchPrompt | E | Bench + O + prompt | C-O | R-O | D-O |
| 40 | orchestration.recover | E | Bench + O | C-O | R-O | D-O |
| 41 | orchestration.createChildTask | E | RunOwner + O + 기동 | C-O | R-A | D-A |
| 42 | orchestration.assignChildTask | E | RunOwner + O + 기동 | C-O | R-A | D-A |
| 43 | orchestration.sendChildMessage | E | RunOwner + O + prompt | C-O | R-A | D-A |
| 44 | orchestration.collectChildResults | E | RunOwner + O | C-O | R-A | D-A |
| 45 | orchestration.interruptChildTask | E | RunOwner + O + 엔진 | C-O | R-A | D-A |
| 46 | orchestration.cancelChildTask | E | RunOwner + O + 엔진 | C-O | R-A | D-A |
| 47 | orchestration.retryChildTask | E | RunOwner + O + 기동 | C-O | R-A | D-A |
| 48 | orchestration.reassignChildTask | E | RunOwner + O + 기동 | C-O | R-A | D-A |
| 49 | orchestration.reportProgress | E | RunOwner + O | C-O | R-A | D-A |
| 50 | orchestration.reportResult | E | RunOwner + O + 알림 | C-O | R-A | D-A |
| 51 | orchestration.requestParentInput | E | RunOwner + O + 알림 | C-O | R-A | D-A |
| 52 | orchestration.reportBlocked | E | RunOwner + O + 알림 | C-O | R-A | D-A |
| 53 | orchestration.sendParentMessage | E | RunOwner + O + prompt | C-O | R-A | D-A |

## 한계(보고 대상)

- 세대 멱등(E) operation의 "중단"은 세대와 함께 멱등 기록이 사라지는 설계라 재시작 뒤 재시도가 `notFound`(작업대 없음)로 끝나는 것이 판정이다. orchestration 파일은 O 원자 쓰기로 "반영 전/후" 둘 중 하나로 남고, 작업대가 닫혀 복구 가능이 된다. 사용자가 복구 뒤 새 키로 다시 요청한다.
- run 제어·교환·orchestration의 개별 operation마다 단절 시험을 두지 않았다. 같은 `EpochHandler` 범위(Bench·RunOwner)를 쓰는 대표 시험(D-E·D-O·D-A)과, 범위와 무관한 H 분리 실행 변이로 덮는다. 엔진 부작용 종류(prompt·기동·취소)는 대표 시험이 prompt(D-E·D-O)와 기동(D-A·D-R)을 직접 잰다. 취소 계열은 자연 멱등(이미 취소된 run 재취소)에 기댄다.
- 실제 git 명령 도중의 프로세스 종료는 C-G의 pending 경로 관찰 판정으로 대신한다(프로세스 kill 시험 없음).
