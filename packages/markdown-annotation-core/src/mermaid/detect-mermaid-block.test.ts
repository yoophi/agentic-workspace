import { describe, expect, it } from "vitest";

import { MERMAID_START_TOKENS, detectMermaidBlock } from "./detect-mermaid-block";

describe("detectMermaidBlock", () => {
  it("detects mermaid language markers regardless of casing and whitespace", () => {
    expect(
      detectMermaidBlock({
        language: " Mermaid ",
        content: "not a known start token",
      }),
    ).toMatchObject({
      detected: true,
      reason: "language-marker",
      declaration: "mermaid",
      source: "not a known start token",
    });
  });

  it("detects every priority start token on the first non-empty line", () => {
    for (const token of MERMAID_START_TOKENS) {
      expect(
        detectMermaidBlock({
          content: `\n  ${token} sample\n    A --> B`,
        }),
      ).toMatchObject({
        detected: true,
        reason: "leading-declaration",
        declaration: token,
      });
    }
  });

  it("does not detect ordinary code or tokens embedded inside other words", () => {
    expect(
      detectMermaidBlock({
        language: "ts",
        content: "const flowchart = true;",
      }),
    ).toBeUndefined();

    expect(
      detectMermaidBlock({
        content: "flowchartFactory()",
      }),
    ).toBeUndefined();
  });

  it("does not detect empty code blocks without a mermaid language marker", () => {
    expect(
      detectMermaidBlock({
        content: "\n  \n",
      }),
    ).toBeUndefined();
  });
});
