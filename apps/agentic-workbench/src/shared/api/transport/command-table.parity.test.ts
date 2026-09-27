// 043 T020: TS command 표가 compat Rust와 같은 operation 입력을 만든다. 기대값은 golden 파일이며, 그 파일은 AW Rust 시험
// (`inbound/compat_parity_tests.rs`)이 compat의 실제 인자 타입·입력 생성 함수로 다시 계산해 지킨다. DTO를 통째로 넘기는
// command는 화면 원본(`wire`)을 보내고, Rust 시험이 그것과 compat 입력이 서버 DTO로 같음을 확인한다.
import { describe, expect, it } from "vitest";

import golden from "./compat-parity.golden.json";
import { COMMANDS } from "./command-table";

type Case = {
  command: string;
  args: Record<string, unknown>;
  benchId: string | null;
  expected: { operation: string; input: unknown; wire?: unknown };
};

const cases = golden.cases as Case[];

describe("command table parity with the compat Rust input builders", () => {
  it.each(cases.map((item, index) => [`${index} ${item.command}`, item] as const))("%s", (_label, item) => {
    const spec = COMMANDS[item.command];
    expect(spec, `table has ${item.command}`).toBeDefined();
    expect(spec.operation).toBe(item.expected.operation);
    const built = spec.input(item.args, item.benchId ?? undefined);
    expect(built).toStrictEqual(item.expected.wire ?? item.expected.input);
  });

  it("covers exactly the commands of the golden file", () => {
    expect(Object.keys(COMMANDS).sort()).toEqual([...new Set(cases.map((item) => item.command))].sort());
  });
});
