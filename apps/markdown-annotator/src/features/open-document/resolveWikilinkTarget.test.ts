import { describe, expect, it } from "vitest";
import { resolveWikilinkTarget } from "./resolveWikilinkTarget";

describe("resolveWikilinkTarget", () => {
  it("resolves a markdown sibling relative to the current document", () => {
    expect(resolveWikilinkTarget("/repo/specs/029/spec.md", "./plan.md")).toEqual({
      fileName: "plan.md",
      path: "/repo/specs/029/plan.md",
    });
  });

  it("decodes spaces and Korean file names", () => {
    expect(resolveWikilinkTarget("examples/spec.md", "./%EB%A7%81%ED%81%AC%20%EB%AC%B8%EC%84%9C.md")).toEqual({
      fileName: "링크 문서.md",
      path: "examples/링크 문서.md",
    });
  });

  it.each(["../secret.md", "./../secret.md", "/absolute.md", "./nested/plan.md", "https://a.b/x.md", "./note.txt"])(
    "rejects unsafe or unsupported target %s",
    (href) => expect(() => resolveWikilinkTarget("/repo/spec.md", href)).toThrow(),
  );
});
