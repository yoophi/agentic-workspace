# 비우기(drain) 입구 분류

`draining` 상태에서 Workbench 호출 입구(`WorkbenchRuntime::call`)의 판정표. research R7.

- **Q**: 조회. 받는다. operation 종류가 query면 자동으로 Q다.
- **C**: 끝내는·해제하는 제어. 받는다. 활성 작업을 줄이거나 이미 있는 작업을 마무리한다.
- **K**: 이어 가기(조건부). 입력이 이미 있는 대기 항목을 가리키고 서버가 확인하면 받는다. 아니면 N.
- **N**: 새 작업·새 데이터 변경. `draining` fault(`outcome: notApplied`)로 거절한다.

`stopping` 상태에서는 분류와 상관없이 새 호출을 모두 `503`으로 거절한다(042).

이 표는 시험이 파싱해 core `drain_class`, `OperationId::ALL`, operation 종류와 대조한다. 형식(열 순서·백틱)을 바꾸면 시험도 바꾼다.

| operation | 종류 | 분류 | 까닭 |
|---|---|---|---|
| `agent.list` | query | Q | 조회 |
| `agent.listProviderSessions` | query | Q | 조회 |
| `agentRunSettings.get` | query | Q | 조회 |
| `agentRunSettings.save` | command | N | 새 작업 또는 새 데이터 변경 |
| `bench.close` | command | C | 작업대의 run을 취소하고 자원을 푼다(활성 작업 감소) |
| `bench.open` | command | N | 새 작업 또는 새 데이터 변경 |
| `bench.requestTitle` | command | C | 실행 중 agent의 표현 요청(알림 발행만). 새 작업 없음 |
| `exchange.acknowledge` | command | C | 이미 요청된 교환의 전달 확인(교환 종결) |
| `exchange.getForRun` | query | Q | 조회 |
| `exchange.list` | query | Q | 조회 |
| `exchange.listPeers` | query | Q | 조회 |
| `exchange.send` | command | N | 새 작업 또는 새 데이터 변경 |
| `exchange.sendFromRun` | command | N | 새 작업 또는 새 데이터 변경 |
| `exchange.syncWorkspace` | command | N | 새 작업 또는 새 데이터 변경 |
| `git.createWorktree` | command | N | 새 작업 또는 새 데이터 변경 |
| `git.deleteWorktree` | command | N | 새 작업 또는 새 데이터 변경 |
| `git.listBranches` | query | Q | 조회 |
| `git.listRemotes` | query | Q | 조회 |
| `git.listWorktrees` | query | Q | 조회 |
| `goal.clear` | command | N | 새 작업 또는 새 데이터 변경 |
| `goal.create` | command | N | 새 작업 또는 새 데이터 변경 |
| `goal.get` | query | Q | 조회 |
| `goal.recordProgress` | command | C | 실행 중 agent의 진행 보고. 새 turn을 만들지 않는다 |
| `goal.update` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.adoptManualChild` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.assignChildTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.bindCoordinator` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.bootstrap` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.cancelChildTask` | command | C | 자식 task 취소(활성 작업 감소) |
| `orchestration.cancelTask` | command | C | task 취소(활성 작업 감소) |
| `orchestration.collectChildResults` | command | C | 끝난 자식 결과 수집(소비) |
| `orchestration.collectReports` | query | Q | 조회 |
| `orchestration.createChildTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.delegateGoal` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.dispatchPrompt` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.get` | query | Q | 조회 |
| `orchestration.getAgentRole` | query | Q | 조회 |
| `orchestration.getOwnTask` | query | Q | 조회 |
| `orchestration.handoffCoordinator` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.interruptChildTask` | command | C | 자식 task 중단(활성 작업 감소) |
| `orchestration.listChildTasks` | query | Q | 조회 |
| `orchestration.listRecoverable` | query | Q | 조회 |
| `orchestration.listTasks` | query | Q | 조회 |
| `orchestration.reassignChildTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.reassignTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.recover` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.reportBlocked` | command | C | 실행 중 자식의 막힘 보고(task 종결 경로) |
| `orchestration.reportProgress` | command | C | 실행 중 자식의 진행 보고 |
| `orchestration.reportResult` | command | C | 실행 중 자식의 결과 보고(task 종결) |
| `orchestration.requestParentInput` | command | C | 실행 중 자식의 입력 요청. 대기가 생기지만 respondInput(C)·취소(C)로 풀 수 있다. 거절하면 자식이 끝낼 길을 잃는다 |
| `orchestration.respondInput` | command | C | 자식의 입력 요청에 답한다(대기 해소) |
| `orchestration.retryChildTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.retryTask` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.sendChildCommand` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.sendChildMessage` | command | N | 새 작업 또는 새 데이터 변경 |
| `orchestration.sendParentMessage` | command | C | 실행 중 자식의 부모 보고. 알림 전달기가 coordinator turn을 만들 수 있으나 그 turn 안의 새 작업 호출은 N |
| `orchestration.setPresentation` | command | C | 표시 상태만 바꾼다. 새 작업 없음 |
| `orchestration.waitChildTasks` | query | Q | 조회 |
| `project.create` | command | N | 새 작업 또는 새 데이터 변경 |
| `project.delete` | command | N | 새 작업 또는 새 데이터 변경 |
| `project.list` | query | Q | 조회 |
| `project.update` | command | N | 새 작업 또는 새 데이터 변경 |
| `run.cancel` | command | C | run 취소(활성 작업 감소) |
| `run.cancelAndSend` | command | N | 새 작업 또는 새 데이터 변경 |
| `run.listToolCandidates` | query | Q | 조회 |
| `run.replay` | query | Q | 조회 |
| `run.respondPermission` | command | C | 권한 대기에 답한다(현재 turn을 끝까지 가게 함) |
| `run.sendPrompt` | command | K | `continuation.exchangeRequestId`가 호출자 작업대의 확인 전 교환이고 대상 run이 이 run이면 받는다(교환 전달). 아니면 N |
| `run.setPermissionMode` | command | C | 권한 대기를 자동 승인·거절로 풀 수 있다(대기 해소) |
| `run.start` | command | N | 새 작업 또는 새 데이터 변경 |
| `run.steer` | command | N | 새 작업 또는 새 데이터 변경 |
| `savedPrompt.create` | command | N | 새 작업 또는 새 데이터 변경 |
| `savedPrompt.delete` | command | N | 새 작업 또는 새 데이터 변경 |
| `savedPrompt.list` | query | Q | 조회 |
| `savedPrompt.update` | command | N | 새 작업 또는 새 데이터 변경 |
| `system.describe` | query | Q | 조회 |
| `worktree.getChanges` | query | Q | 조회 |
| `worktree.getCommitDetail` | query | Q | 조회 |
| `worktree.getCommitFileDiff` | query | Q | 조회 |
| `worktree.getFileDiff` | query | Q | 조회 |
| `worktree.getGraph` | query | Q | 조회 |
| `worktree.listChanges` | query | Q | 조회 |
| `worktree.listFiles` | query | Q | 조회 |
| `worktree.listHistory` | query | Q | 조회 |
| `worktree.readTextFile` | query | Q | 조회 |

## 새 operation(044)

| operation | 종류 | 분류 | 까닭 |
|---|---|---|---|
| `server.status` | query | Q | 조회 |
| `server.stop` | command | C | 정지 요청(소유자) |
| `lease.acquire` | command | C | 임대는 유휴 판정에만 쓰인다. 비우기 중에도 받는다(유휴 비우기는 임대가 잡히면 서빙으로 돌아간다) |
| `lease.renew` | command | C | 같음 |
| `lease.release` | command | C | 같음 |
| `desktop.issueWindowToken` | command | C | 비우기 중에도 창이 다시 붙어 권한 응답·취소를 할 수 있어야 한다 |
| `desktop.retireWindow` | command | C | 토큰 폐기·작업대 닫기(활성 작업 감소) |
| `bench.list` | query | Q | 조회 |

## R7-check 결과

(구현 전 기록) 대기 중 자식 명령(`delivery: queue`)의 전달 경로: 서버 내부 / 클라이언트 호출 필요. 근거 코드 위치와 분류 반영.
