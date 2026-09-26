import { expectTypeOf, test } from "vitest";

import type {
  CallReply,
  DescribeOutput,
  OperationId,
  OperationMap,
  Project,
} from "./operation-map";

test("operation ids are exactly the three registered operations", () => {
  expectTypeOf<OperationId>().toEqualTypeOf<
    "project.list" | "project.create" | "system.describe"
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

test("system.describe returns DescribeOutput", () => {
  expectTypeOf<OperationMap["system.describe"]["output"]>().toEqualTypeOf<DescribeOutput>();
});

test("mismatched output types are compile errors", () => {
  // @ts-expect-error project.create의 output은 Project 하나이며 배열이 아니다
  const wrong: OperationMap["project.create"]["output"] = [] as Project[];
  // @ts-expect-error project.list의 output은 배열이며 단일 Project가 아니다
  const alsoWrong: OperationMap["project.list"]["output"] = {} as Project;
  void wrong;
  void alsoWrong;
});

test("generic CallReply output is unconstrained JSON and Accepted uses camelCase", () => {
  expectTypeOf<Extract<CallReply, { kind: "complete" }>["output"]>().toBeUnknown();
  expectTypeOf<Extract<CallReply, { kind: "accepted" }>>().toHaveProperty("executionId");
});
