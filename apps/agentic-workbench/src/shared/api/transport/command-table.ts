// 호환 command ↔ Workbench operation 표(043 T024). HttpTransport는 화면이 오늘 부르는 command 이름·인자를 그대로 받아
// 이 표로 operation 입력을 만들고 결과·오류를 오늘과 같은 모양으로 돌려준다. 규칙의 정본은 compat Rust다
// (`inbound/tauri_commands.rs`·`inbound/workbench_compat.rs`) — 입력 변환은 golden 파일(`compat-parity.golden.json`)을
// Rust·TS 시험이 함께 대조하고, 작업대 없음·결과 변환·오류 문자열은 표 옆 주석의 Rust 규칙과 단위 시험이 지킨다.
import type { FaultFlavor, OperationId } from "@yoophi/workbench-client";

type Args = Record<string, unknown>;

/** 작업대가 필요한 방식. `ensure`: 없으면 연다(hint = 창 경로가 없을 때 쓸 작업 디렉터리). `lookup`: 있을 때만. */
export type BenchMode =
  | { kind: "none" }
  | { kind: "ensure"; hint?: (args: Args) => string | null | undefined }
  | { kind: "lookup"; missing: { result: (args: Args) => unknown } | { error: (args: Args) => string } };

export interface CommandSpec {
  operation: OperationId;
  bench: BenchMode;
  flavor?: FaultFlavor;
  input: (args: Args, benchId: string | undefined) => Record<string, unknown>;
  /** 결과 변환(없으면 output 그대로). `windowLabel`은 orchestration 묶임 표시용. */
  output?: (output: unknown, context: { windowLabel: string; args: Args }) => unknown;
  /** operation 실패를 결과로 바꾸는 command(`replay_orchestration_runtime_events`는 항상 RunReplay). */
  onFault?: (args: Args) => unknown;
}

const obj = (value: unknown) => (value ?? {}) as Args;
const optional = (target: Args, key: string, value: unknown) => {
  if (value !== undefined && value !== null) {
    target[key] = value;
  }
  return target;
};
const wd = (args: Args) => ({ workingDirectory: args.workingDirectory });

// Rust `session_for_window`: eventStreamId 제거, boundWindowLabel = 창 label.
export function sessionForWindow(session: unknown, windowLabel: string): unknown {
  if (session === null || typeof session !== "object" || Array.isArray(session)) {
    return session;
  }
  const { eventStreamId: _dropped, ...rest } = session as Args;
  return { ...rest, boundWindowLabel: windowLabel };
}

const bound = (output: unknown, { windowLabel }: { windowLabel: string }) => sessionForWindow(output, windowLabel);
const ensure = (hint?: (args: Args) => string | null | undefined): BenchMode => ({ kind: "ensure", hint });
const orchestrationRequest = (operation: OperationId, output = bound): CommandSpec => ({
  operation,
  bench: ensure(),
  flavor: "orchestration",
  input: (args, benchId) => ({ benchId, request: args.input }),
  output,
});

function exchangeError(code: string, message: string) {
  return JSON.stringify({ code, message });
}

export const COMMANDS: Record<string, CommandSpec> = {
  // ---- 전역 데이터(작업대 없음) ----
  list_projects: { operation: "project.list", bench: { kind: "none" }, input: () => ({}) },
  create_project: {
    operation: "project.create",
    bench: { kind: "none" },
    input: (args) => {
      const input = obj(args.input);
      return { name: input.name, workingDirectory: input.workingDirectory, description: input.description ?? null };
    },
  },
  update_project: {
    operation: "project.update",
    bench: { kind: "none" },
    input: (args) => {
      const input = obj(args.input);
      return {
        id: args.id,
        name: input.name,
        workingDirectory: input.workingDirectory,
        description: input.description ?? null,
      };
    },
  },
  delete_project: { operation: "project.delete", bench: { kind: "none" }, input: (args) => ({ id: args.id }) },
  list_saved_prompts: { operation: "savedPrompt.list", bench: { kind: "none" }, input: () => ({}) },
  create_saved_prompt: {
    operation: "savedPrompt.create",
    bench: { kind: "none" },
    input: (args) => ({ label: obj(args.input).label, prompt: obj(args.input).prompt }),
  },
  update_saved_prompt: {
    operation: "savedPrompt.update",
    bench: { kind: "none" },
    input: (args) => ({ id: args.id, label: obj(args.input).label, prompt: obj(args.input).prompt }),
  },
  delete_saved_prompt: { operation: "savedPrompt.delete", bench: { kind: "none" }, input: (args) => ({ id: args.id }) },
  get_goal: { operation: "goal.get", bench: { kind: "none" }, input: wd },
  create_goal: {
    operation: "goal.create",
    bench: { kind: "none" },
    input: (args) => {
      const input = obj(args.input);
      return optional({ workingDirectory: input.workingDirectory, objective: input.objective }, "tokenBudget", input.tokenBudget);
    },
  },
  update_goal: {
    operation: "goal.update",
    bench: { kind: "none" },
    input: (args) => {
      const input = obj(args.input);
      const value: Args = { workingDirectory: args.workingDirectory };
      optional(value, "objective", input.objective);
      optional(value, "status", input.status);
      // compat 인자 타입은 `Option<Option<usize>>`지만 serde 기본 역직렬화는 JSON null을 바깥 `None`으로 읽는다 — 그래서
      // null은 생략되고 값만 전달된다(golden이 고정). 043은 오늘 동작을 바꾸지 않는다(관찰: implementation-evidence T020).
      return optional(value, "tokenBudget", input.tokenBudget);
    },
  },
  clear_goal: { operation: "goal.clear", bench: { kind: "none" }, input: wd },
  record_goal_progress: {
    operation: "goal.recordProgress",
    bench: { kind: "none" },
    input: (args) => ({
      workingDirectory: args.workingDirectory,
      tokensUsed: obj(args.input).tokensUsed,
      timeUsedSeconds: obj(args.input).timeUsedSeconds,
    }),
  },
  get_agent_run_settings: { operation: "agentRunSettings.get", bench: { kind: "none" }, input: wd },
  save_agent_run_settings: {
    operation: "agentRunSettings.save",
    bench: { kind: "none" },
    input: (args) => ({ settings: args.settings }),
  },
  list_git_remotes: { operation: "git.listRemotes", bench: { kind: "none" }, input: wd },
  list_git_branches: { operation: "git.listBranches", bench: { kind: "none" }, input: wd },
  list_git_worktrees: {
    operation: "git.listWorktrees",
    bench: { kind: "none" },
    input: (args) => optional(wd(args), "includeStatus", args.includeStatus),
  },
  list_worktree_changes: { operation: "worktree.listChanges", bench: { kind: "none" }, input: wd },
  create_git_worktree: {
    operation: "git.createWorktree",
    bench: { kind: "none" },
    input: (args) => {
      const draft = obj(args.input);
      const value = { workingDirectory: args.workingDirectory, path: draft.path } as Args;
      optional(value, "branch", draft.branch);
      return optional(value, "reference", draft.reference);
    },
  },
  delete_git_worktree: {
    operation: "git.deleteWorktree",
    bench: { kind: "none" },
    input: (args) => ({ workingDirectory: args.workingDirectory, path: args.path }),
  },
  get_worktree_changes: { operation: "worktree.getChanges", bench: { kind: "none" }, input: wd },
  get_worktree_file_diff: {
    operation: "worktree.getFileDiff",
    bench: { kind: "none" },
    input: (args) => ({ workingDirectory: args.workingDirectory, path: args.path }),
  },
  list_worktree_files: {
    operation: "worktree.listFiles",
    bench: { kind: "none" },
    input: (args) => {
      const value = wd(args) as Args;
      const scope = args.scope as Args | null | undefined;
      if (scope) {
        const scoped: Args = { kind: scope.kind };
        optional(scoped, "dir", scope.dir);
        optional(scoped, "depth", scope.depth);
        value.scope = scoped;
      }
      return value;
    },
  },
  read_worktree_text_file: {
    operation: "worktree.readTextFile",
    bench: { kind: "none" },
    input: (args) => ({ workingDirectory: args.workingDirectory, path: args.path }),
  },
  list_worktree_git_history: {
    operation: "worktree.listHistory",
    bench: { kind: "none" },
    input: (args) => optional(optional(optional(wd(args), "maxCount", args.maxCount), "offset", args.offset), "cursor", args.cursor),
  },
  get_worktree_git_graph: {
    operation: "worktree.getGraph",
    bench: { kind: "none" },
    input: (args) => optional(optional(optional(wd(args), "maxCount", args.maxCount), "offset", args.offset), "cursor", args.cursor),
  },
  get_worktree_commit_detail: {
    operation: "worktree.getCommitDetail",
    bench: { kind: "none" },
    input: (args) => ({ workingDirectory: args.workingDirectory, commitHash: args.commitHash }),
  },
  get_worktree_commit_file_diff: {
    operation: "worktree.getCommitFileDiff",
    bench: { kind: "none" },
    input: (args) => ({ workingDirectory: args.workingDirectory, commitHash: args.commitHash, path: args.path }),
  },
  list_agents: { operation: "agent.list", bench: { kind: "none" }, input: () => ({}), onFault: () => [] },
  list_provider_sessions: {
    operation: "agent.listProviderSessions",
    bench: { kind: "none" },
    input: (args) => optional({ agentId: args.agentId }, "cwd", args.cwd),
  },

  // ---- run(작업대) ----
  start_agent_run: {
    operation: "run.start",
    bench: ensure((args) => obj(args.request).cwd as string | undefined),
    input: (args, benchId) => optional({ benchId, request: args.request }, "panelId", args.panelId),
  },
  list_agent_tool_command_candidates: {
    operation: "run.listToolCandidates",
    bench: ensure((args) => obj(args.input).workingDirectory as string | undefined),
    input: (args, benchId) => ({ benchId, query: args.input }),
  },
  send_prompt_to_run: runPrompt("run.sendPrompt"),
  steer_prompt_to_run: runPrompt("run.steer"),
  cancel_current_prompt_and_send_to_run: runPrompt("run.cancelAndSend"),
  set_run_permission_mode: {
    operation: "run.setPermissionMode",
    bench: { kind: "lookup", missing: { error: () => "agent run is not active" } },
    input: (args, benchId) => ({ benchId, runId: args.runId, mode: args.permissionMode }),
  },
  cancel_agent_run: {
    operation: "run.cancel",
    bench: { kind: "lookup", missing: { result: () => null } },
    input: (args, benchId) => ({ benchId, runId: args.runId }),
  },
  respond_agent_permission: {
    operation: "run.respondPermission",
    bench: { kind: "lookup", missing: { error: (args) => `unknown or finished run: ${String(args.runId)}` } },
    input: (args, benchId) => ({ benchId, runId: args.runId, permissionId: args.permissionId, optionId: args.optionId }),
  },

  // ---- 교환(작업대) ----
  sync_agent_workspace: {
    operation: "exchange.syncWorkspace",
    bench: ensure((args) => obj(args.request).worktreePath as string | undefined),
    flavor: "exchange",
    input: (args, benchId) => ({ benchId, request: args.request }),
  },
  send_agent_exchange: {
    operation: "exchange.send",
    bench: { kind: "lookup", missing: { error: () => exchangeError("unknownWorkspace", "Agent workspace is not registered.") } },
    flavor: "exchange",
    input: (args, benchId) => ({ benchId, request: args.request }),
  },
  acknowledge_agent_exchange: {
    operation: "exchange.acknowledge",
    bench: { kind: "lookup", missing: { error: () => exchangeError("unknownExchange", "Exchange was not found.") } },
    flavor: "exchange",
    input: (args, benchId) => ({ benchId, request: args.request }),
  },
  list_agent_exchanges: {
    operation: "exchange.list",
    bench: { kind: "lookup", missing: { result: () => [] } },
    flavor: "exchange",
    input: (_args, benchId) => ({ benchId }),
  },

  // ---- orchestration(작업대) ----
  bootstrap_orchestration_workspace: {
    operation: "orchestration.bootstrap",
    bench: ensure((args) => obj(args.input).worktreePath as string | undefined),
    flavor: "orchestration",
    input: (args, benchId) => ({
      benchId,
      worktreePath: obj(args.input).worktreePath,
      resumeWorkspaceId: obj(args.input).resumeWorkspaceId ?? null,
    }),
    output: bound,
  },
  get_orchestration_workspace: {
    operation: "orchestration.get",
    bench: { kind: "lookup", missing: { result: () => null } },
    flavor: "orchestration",
    input: (_args, benchId) => ({ benchId }),
    output: (output, context) => (output === null ? null : bound(output, context)),
  },
  list_recoverable_orchestration_workspaces: {
    operation: "orchestration.listRecoverable",
    bench: ensure((args) => obj(args.input).worktreePath as string | undefined),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, worktreePath: obj(args.input).worktreePath }),
  },
  bind_main_coordinator_run: orchestrationRequest("orchestration.bindCoordinator"),
  set_orchestration_presentation: orchestrationRequest("orchestration.setPresentation"),
  cancel_orchestration_task: orchestrationRequest("orchestration.cancelTask"),
  retry_orchestration_task: orchestrationRequest("orchestration.retryTask"),
  reassign_orchestration_task: orchestrationRequest("orchestration.reassignTask"),
  handoff_orchestration_coordinator: orchestrationRequest("orchestration.handoffCoordinator"),
  delegate_orchestration_goal: {
    operation: "orchestration.delegateGoal",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, request: args.input }),
  },
  adopt_manual_orchestration_child: {
    operation: "orchestration.adoptManualChild",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, panelId: obj(args.input).panelId, title: obj(args.input).title }),
    output: bound,
  },
  list_orchestration_tasks: {
    operation: "orchestration.listTasks",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, generationId: obj(args.input).generationId }),
  },
  collect_orchestration_reports: {
    operation: "orchestration.collectReports",
    bench: { kind: "lookup", missing: { result: () => [] } },
    flavor: "orchestration",
    input: (_args, benchId) => ({ benchId }),
  },
  send_orchestration_child_command: {
    operation: "orchestration.sendChildCommand",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, input: args.input }),
  },
  respond_orchestration_input: {
    operation: "orchestration.respondInput",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, request: args.input }),
  },
  replay_orchestration_runtime_events: {
    operation: "run.replay",
    bench: { kind: "lookup", missing: { result: (args) => missingReplay(args) } },
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, runId: obj(args.input).runId, afterSequence: obj(args.input).afterSequence }),
    // 오늘처럼 항상 RunReplay: 작업대가 없거나 재생할 수 없으면 Missing 형태.
    onFault: (args) => missingReplay(args),
  },
  dispatch_orchestration_prompt: {
    operation: "orchestration.dispatchPrompt",
    bench: ensure(),
    flavor: "orchestration",
    input: (args, benchId) => ({ benchId, request: args.input }),
  },
  recover_orchestration_workspace: {
    operation: "orchestration.recover",
    bench: ensure(),
    flavor: "orchestration",
    input: (_args, benchId) => ({ benchId }),
    output: bound,
  },
};

export function missingReplay(args: Args) {
  const input = obj(args.input);
  return {
    runId: input.runId,
    events: [],
    lastSequence: 0,
    terminal: false,
    gapDetected: Number(input.afterSequence ?? 0) > 0,
  };
}

function runPrompt(operation: OperationId): CommandSpec {
  return {
    operation,
    bench: { kind: "lookup", missing: { error: () => "agent run is not active" } },
    input: (args, benchId) => ({ benchId, runId: args.runId, prompt: args.prompt }),
  };
}
