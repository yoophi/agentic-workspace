import { expectTypeOf, test } from "vitest";

import type {
  AgentRunSettings,
  CallReply,
  DescribeOutput,
  Goal,
  OperationId,
  OperationMap,
  Project,
  SavedPrompt,
} from "./operation-map";

test("operation ids are exactly the registered operations (037 + 038 US1)", () => {
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
  void wrong;
  void alsoWrong;
  void crossDomain;
});

test("generic CallReply output is unconstrained JSON and Accepted uses camelCase", () => {
  expectTypeOf<Extract<CallReply, { kind: "complete" }>["output"]>().toBeUnknown();
  expectTypeOf<Extract<CallReply, { kind: "accepted" }>>().toHaveProperty("executionId");
});
