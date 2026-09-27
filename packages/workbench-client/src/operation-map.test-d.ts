import { expectTypeOf, test } from "vitest";

import type {
  AgentDescriptor,
  AgentRunSettings,
  CallReply,
  DescribeOutput,
  EventFrame,
  EventMap,
  EventSchemaId,
  GitCommitHistory,
  GitRemote,
  GitWorktree,
  Goal,
  OperationId,
  OperationMap,
  Project,
  ProviderSession,
  SavedPrompt,
  WorktreeFileEntry,
  WorktreeTextFile,
} from "./operation-map";

test("operation ids are exactly the registered operations (037 + 038 + 040)", () => {
  expectTypeOf<OperationId>().toEqualTypeOf<
    | "project.list"
    | "project.create"
    | "project.update"
    | "project.delete"
    | "savedPrompt.list"
    | "savedPrompt.create"
    | "savedPrompt.update"
    | "savedPrompt.delete"
    | "goal.get"
    | "goal.create"
    | "goal.update"
    | "goal.clear"
    | "goal.recordProgress"
    | "agentRunSettings.get"
    | "agentRunSettings.save"
    | "git.listRemotes"
    | "git.listBranches"
    | "git.listWorktrees"
    | "git.createWorktree"
    | "git.deleteWorktree"
    | "worktree.listChanges"
    | "worktree.getChanges"
    | "worktree.getFileDiff"
    | "worktree.listFiles"
    | "worktree.readTextFile"
    | "worktree.listHistory"
    | "worktree.getGraph"
    | "worktree.getCommitDetail"
    | "worktree.getCommitFileDiff"
    | "agent.list"
    | "agent.listProviderSessions"
    | "bench.open"
    | "bench.close"
    | "bench.requestTitle"
    | "system.describe"
  >();
});

test("project.list pairs an empty input with Project[]", () => {
  expectTypeOf<OperationMap["project.list"]["output"]>().toEqualTypeOf<Project[]>();
  expectTypeOf<OperationMap["project.list"]["input"]>().toEqualTypeOf<Record<string, never>>();
});

test("project.create pairs ProjectCreateInput with a single Project", () => {
  expectTypeOf<OperationMap["project.create"]["output"]>().toEqualTypeOf<Project>();
  expectTypeOf<OperationMap["project.create"]["input"]>().toHaveProperty("name");
  expectTypeOf<OperationMap["project.create"]["input"]>().toHaveProperty("workingDirectory");
});

test("project.update/delete: update returns Project, delete returns null", () => {
  expectTypeOf<OperationMap["project.update"]["input"]>().toHaveProperty("id");
  expectTypeOf<OperationMap["project.update"]["output"]>().toEqualTypeOf<Project>();
  expectTypeOf<OperationMap["project.delete"]["output"]>().toEqualTypeOf<null>();
});

test("savedPrompt operations pair with SavedPrompt", () => {
  expectTypeOf<OperationMap["savedPrompt.list"]["output"]>().toEqualTypeOf<SavedPrompt[]>();
  expectTypeOf<OperationMap["savedPrompt.create"]["input"]>().toHaveProperty("label");
  expectTypeOf<OperationMap["savedPrompt.create"]["output"]>().toEqualTypeOf<SavedPrompt>();
  expectTypeOf<OperationMap["savedPrompt.delete"]["output"]>().toEqualTypeOf<null>();
});

test("goal.get is nullable and goal mutations return Goal", () => {
  expectTypeOf<OperationMap["goal.get"]["output"]>().toEqualTypeOf<Goal | null>();
  expectTypeOf<OperationMap["goal.create"]["input"]>().toHaveProperty("objective");
  expectTypeOf<OperationMap["goal.recordProgress"]["input"]>().toHaveProperty("tokensUsed");
  expectTypeOf<OperationMap["goal.update"]["output"]>().toEqualTypeOf<Goal>();
  expectTypeOf<OperationMap["goal.clear"]["output"]>().toEqualTypeOf<null>();
});

test("agentRunSettings.save takes the whole settings object under `settings`", () => {
  expectTypeOf<OperationMap["agentRunSettings.save"]["input"]>().toHaveProperty("settings");
  expectTypeOf<OperationMap["agentRunSettings.save"]["input"]["settings"]>().toEqualTypeOf<AgentRunSettings>();
  expectTypeOf<OperationMap["agentRunSettings.get"]["output"]>().toEqualTypeOf<AgentRunSettings | null>();
});

test("git queries return lists and git changes return null", () => {
  expectTypeOf<OperationMap["git.listRemotes"]["output"]>().toEqualTypeOf<GitRemote[]>();
  expectTypeOf<OperationMap["git.listWorktrees"]["output"]>().toEqualTypeOf<GitWorktree[]>();
  expectTypeOf<OperationMap["git.listWorktrees"]["input"]>().toHaveProperty("includeStatus");
  expectTypeOf<OperationMap["git.createWorktree"]["input"]>().toHaveProperty("path");
  expectTypeOf<OperationMap["git.createWorktree"]["output"]>().toEqualTypeOf<null>();
  expectTypeOf<OperationMap["git.deleteWorktree"]["output"]>().toEqualTypeOf<null>();
});

test("worktree queries: optional scope, text preview, history page", () => {
  type ListFilesInput = OperationMap["worktree.listFiles"]["input"];
  // scope는 생략 가능하다(생략 = 전체 트리).
  const withoutScope: ListFilesInput = { workingDirectory: "/repo" };
  void withoutScope;
  expectTypeOf<OperationMap["worktree.listFiles"]["output"]>().toEqualTypeOf<WorktreeFileEntry[]>();
  expectTypeOf<OperationMap["worktree.readTextFile"]["output"]>().toEqualTypeOf<WorktreeTextFile>();
  expectTypeOf<OperationMap["worktree.listHistory"]["output"]>().toEqualTypeOf<GitCommitHistory>();
  expectTypeOf<OperationMap["worktree.listHistory"]["input"]>().toHaveProperty("cursor");
});

test("agent queries: catalog list and provider sessions with optional cwd", () => {
  expectTypeOf<OperationMap["agent.list"]["output"]>().toEqualTypeOf<AgentDescriptor[]>();
  expectTypeOf<OperationMap["agent.listProviderSessions"]["output"]>().toEqualTypeOf<ProviderSession[]>();
  const withoutCwd: OperationMap["agent.listProviderSessions"]["input"] = { agentId: "codex" };
  void withoutCwd;
});

test("system.describe returns DescribeOutput", () => {
  expectTypeOf<OperationMap["system.describe"]["output"]>().toEqualTypeOf<DescribeOutput>();
});

test("mismatched output types are compile errors", () => {
  // @ts-expect-error project.create의 output은 Project 하나이며 배열이 아니다
  const wrong: OperationMap["project.create"]["output"] = [] as Project[];
  // @ts-expect-error project.list의 output은 배열이며 단일 Project가 아니다
  const alsoWrong: OperationMap["project.list"]["output"] = {} as Project;
  // @ts-expect-error goal.get의 output은 Goal | null이며 SavedPrompt가 아니다
  const crossDomain: OperationMap["goal.get"]["output"] = {} as SavedPrompt;
  // @ts-expect-error git.createWorktree의 output은 null이며 GitWorktree가 아니다
  const gitChange: OperationMap["git.createWorktree"]["output"] = {} as GitWorktree;
  void wrong;
  void alsoWrong;
  void crossDomain;
  void gitChange;
});

test("generic CallReply output is unconstrained JSON and Accepted uses camelCase", () => {
  expectTypeOf<Extract<CallReply, { kind: "complete" }>["output"]>().toBeUnknown();
  expectTypeOf<Extract<CallReply, { kind: "accepted" }>>().toHaveProperty("executionId");
});

test("event schema ids correlate with typed bodies (039)", () => {
  expectTypeOf<EventSchemaId>().toEqualTypeOf<
    "run.event.v1" | "worktree.changed.v1" | "orchestration.workspaceUpdated.v1"
  >();

  const run = {} as EventMap["run.event.v1"];
  if (run.body.type === "agentMessage") {
    expectTypeOf(run.body.text).toEqualTypeOf<string>();
  }
  expectTypeOf<EventMap["worktree.changed.v1"]["body"]["kind"]>().toEqualTypeOf<
    "file" | "git"
  >();
  // @ts-expect-error worktree 알림 본문에는 run 이벤트의 `type`이 없다
  void ({} as EventMap["worktree.changed.v1"]).body.type;
  expectTypeOf<EventMap["orchestration.workspaceUpdated.v1"]["body"]["revision"]>().toEqualTypeOf<number>();
});

test("event frames discriminate on type (039)", () => {
  const frame = {} as EventFrame;
  if (frame.type === "hello") {
    expectTypeOf(frame.epoch).toEqualTypeOf<string>();
  } else if (frame.type === "gap") {
    expectTypeOf(frame.reason).toEqualTypeOf<
      | "unknownStream"
      | "evicted"
      | "epochChanged"
      | "retentionExceeded"
      | "subscriberLagged"
      | "shutdown"
    >();
  }
});
