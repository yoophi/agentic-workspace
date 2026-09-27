# 호환 command 인벤토리 (T002)

근거: `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`(command 본문의 `OperationId`와 작업대 도우미)와 화면 `invoke` 사용처(`apps/agentic-workbench/src/**`)를 대조했다. 기준 main은 b682c6b.

## 표기

- **분류**: S = 서버 소유(네트워크 경로로 옮김), D = 데스크톱 표현(Tauri에 남음)
- **작업대**: 오늘 compat이 창 label로 작업대를 넣는 방식
  - `ensure` = 창 작업대를 열거나 찾음 → 네트워크 경로에서는 `ensure_window_bench`로 id를 받아 입력에 `benchId`를 싣는다
  - `lookup` = 있을 때만 찾음(없으면 오늘 정해진 결과) → 화면이 알고 있는 작업대 id를 쓴다. 없으면 같은 결과를 흉내 낸다
  - `—` = 작업대 없음
- **주체**: 창 주체가 필요한 command(W). 작업대 소유 판정 대상이다. 043에서는 **모든 S command를 창 주체로 부른다**(D2). 작업대와 무관한 조회도 한 경로·한 주체를 유지하기 위해서다.
- **매퍼**: compat이 결과·오류에 적용하는 변환. HttpTransport는 같은 변환을 TS로 재현하고, 동등성 시험(T020)이 이를 지킨다.

## 서버 소유 (S)

| 저장소 (`entities/*/api`) | command | operation | 작업대 | 매퍼 |
|---|---|---|---|---|
| project-repository | list_projects / create_project / update_project / delete_project | project.list / create / update / delete | — | fault.message |
| saved-prompt-repository | list/create/update/delete_saved_prompt | savedPrompt.* | — | fault.message |
| goal-repository | get/create/update/clear_goal, record_goal_progress | goal.* | — | fault.message |
| agent-run-repository | get/save_agent_run_settings | agentRunSettings.get / save | — | fault.message |
| agent-run-repository | list_agents, list_provider_sessions | agent.list / listProviderSessions | — | fault.message |
| agent-run-repository | start_agent_run | run.start | ensure(hint `request.cwd`) | fault.message |
| agent-run-repository | list_agent_tool_command_candidates | run.listToolCandidates | ensure(hint cwd) | fault.message |
| agent-run-repository | send_prompt_to_run / steer_prompt_to_run / cancel_current_prompt_and_send_to_run | run.sendPrompt / steer / cancelAndSend | lookup(없으면 "agent run is not active") | fault.message |
| agent-run-repository | set_run_permission_mode | run.setPermissionMode | lookup(같은 문구) | fault.message |
| agent-run-repository | cancel_agent_run, respond_agent_permission | run.cancel / respondPermission | lookup | fault.message |
| agent-exchange-repository | sync_agent_workspace | exchange.syncWorkspace | ensure(hint worktreePath) | 교환 오류 JSON `{code,message}` |
| agent-exchange-repository | send_agent_exchange / acknowledge_agent_exchange / list_agent_exchanges | exchange.send / acknowledge / list | lookup | 교환 오류 JSON |
| orchestration-repository | bootstrap_orchestration_workspace, adopt_manual_orchestration_child, recover_orchestration_workspace | orchestration.bootstrap / adoptManualChild / recover | ensure | `session_for_window`(`eventStreamId` 제거, `boundWindowLabel` = 창 label) + orchestration 오류(`details.orchestrationError` JSON, 없으면 message) |
| orchestration-repository | bind_main_coordinator_run, set_orchestration_presentation, cancel/retry/reassign task, handoff_orchestration_coordinator | orchestration.bindCoordinator / setPresentation / cancelTask / retryTask / reassignTask / handoffCoordinator | ensure(입력 `{benchId, request}`) | session_for_window + orchestration 오류 |
| orchestration-repository | get_orchestration_workspace | orchestration.get | lookup(없으면 `null`) | session_for_window(결과가 null이 아닐 때) |
| orchestration-repository | list_recoverable_orchestration_workspaces, delegate_orchestration_goal, list_orchestration_tasks, send_orchestration_child_command, respond_orchestration_input, dispatch_orchestration_prompt | orchestration.listRecoverable / delegateGoal / listTasks / sendChildCommand / respondInput / dispatchPrompt | ensure | orchestration 오류 |
| orchestration-repository | collect_orchestration_reports, replay_orchestration_runtime_events | orchestration.collectReports / run.replay | lookup | orchestration 오류 |
| git-remote / git-branch / git-worktree-repository | list_git_remotes / list_git_branches / list_git_worktrees / create_git_worktree / delete_git_worktree | git.* | — | fault.message |
| worktree-change(s)-repository | list_worktree_changes / get_worktree_changes / get_worktree_file_diff | worktree.listChanges / getChanges / getFileDiff | — | fault.message |
| worktree-file-repository | list_worktree_files / read_worktree_text_file | worktree.listFiles / readTextFile | — | fault.message |
| worktree-git-repository | list_worktree_git_history / get_worktree_git_graph / get_worktree_commit_detail / get_worktree_commit_file_diff | worktree.listHistory / getGraph / getCommitDetail / getCommitFileDiff | — | fault.message |

## 데스크톱 표현·앱 내부 (D, Tauri에 남음)

| 저장소 | command | 이유 |
|---|---|---|
| appearance-preferences-repository | get_appearance_preferences / set_font_size_step / adjust_font_size_step | 모양 설정(데스크톱 표현) |
| worktree-workspace-layout-repository | get / save_worktree_workspace_layout | Worktree 창 배치 |
| settings-window-repository | open_settings_window | 창 열기 |
| project 화면 | open_worktree_window | 창 열기 |
| shared/api/external-url | open_external_url | 외부 URL |
| (부팅) | get_workbench_connection, ensure_window_bench(신규), declare_network_delivery(신규), withdraw_network_delivery(신규, OCR O5) | 연결 정보·창 작업대·전달 선언·호환 부팅 시 선언 철회 |
| worktree 감시 | start_worktree_watcher / stop_worktree_watcher | 네트워크 경로에서는 `worktree:<path>` 구독으로 대체한다(구독이 감시를 시작함, 039). 호환 경로 창만 오늘 command를 쓴다 |

## 이벤트 (구독 대상)

| 오늘 삽입 경로 이름 | 네트워크 스트림 | 복구(R8) |
|---|---|---|
| agent-run-event-fallback | `run:<runId>` | run.replay 기준점 |
| agent-exchange-requested-fallback / agent-exchange-status-fallback | `exchange:<benchId>` | exchange.list + 재조정 |
| orchestration-workspace/command/notification-updated-fallback | `orchestration:<binding>` | orchestration.get |
| mcp-window-title-fallback | `bench:<benchId>` 제목 요청 | 알림(재조회 없음). `set_title` 창 적용은 앱이 계속 한다(표현 상태) |
| worktree-changed(네이티브 listen) | `worktree:<path>` | 재조회 |

## 교환 상태 의미 (T003)

코드 근거는 `crates/workbench-core/src/application/agent_exchange_service.rs`의 `send`와 `acknowledge`다.

- `send`는 교환을 `Accepted`로 저장하고 `emit_requested`(requested 이벤트)를 낸 뒤 status 이벤트를 낸다. requested 발행에 실패하면 곧바로 `Failed`로 바꾼다. 같은 `requestId`로 다시 보내면 저장된 값을 그대로 돌려준다(`StoreExchangeOutcome::Existing`).
- `acknowledge`는 종결 결과(`Delivered`·`Rejected`·`Failed`·`Cancelled`)만 받는다. 현재 상태와 같은 결과가 다시 오면 값만 돌려주고 이벤트를 내지 않는다(요청 id 기준 멱등). 대상 패널이 다르면 `unknownTarget`으로 거절한다.
- `Pending`은 이 경로에서 쓰지 않는다.
- **재조정 대상**은 `exchange.list`에서 `status == Accepted`이면서 `target.panelId`가 이 창의 패널인 교환이다. `Accepted`는 "requested 이벤트를 냈지만 화면의 ack를 아직 받지 못함"을 뜻한다. 종결 상태인 교환은 재조정하지 않는다.
