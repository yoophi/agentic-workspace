// 043 T027 도구 검증: 가짜 서버의 역변환이 정확해야 화면 시험의 호출 기록 단정이 두 경로에서 같은 뜻이다. golden 사례마다
// 정변환(command 표) ∘ 역변환 = 항등을 확인한다.
import { describe, expect, it } from "vitest";

import golden from "../compat-parity.golden.json";
import { COMMANDS } from "../command-table";

import { reverseArgs } from "./compat-simulating-server";

type Case = { command: string; args: Record<string, unknown>; benchId: string | null; expected: { input: unknown; wire?: unknown } };

describe("compat simulating server reverse mapping", () => {
  it.each((golden.cases as Case[]).map((item, index) => [`${index} ${item.command}`, item] as const))("%s", (_label, item) => {
    const input = (item.expected.wire ?? item.expected.input) as Record<string, unknown>;
    const args = reverseArgs(item.command, input);
    expect(COMMANDS[item.command].input(args, item.benchId ?? undefined)).toStrictEqual(input);
  });
});
