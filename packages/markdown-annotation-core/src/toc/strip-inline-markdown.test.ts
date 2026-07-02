import { describe, expect, it } from "vitest";

import { stripInlineMarkdown } from "./strip-inline-markdown";

describe("stripInlineMarkdown", () => {
  it("passes plain text through unchanged", () => {
    expect(stripInlineMarkdown("Plain heading text")).toBe("Plain heading text");
  });

  it("strips bold markers", () => {
    expect(stripInlineMarkdown("**bold** heading")).toBe("bold heading");
    expect(stripInlineMarkdown("__bold__ heading")).toBe("bold heading");
  });

  it("strips italic markers", () => {
    expect(stripInlineMarkdown("*italic* heading")).toBe("italic heading");
    expect(stripInlineMarkdown("_italic_ heading")).toBe("italic heading");
  });

  it("strips inline code markers", () => {
    expect(stripInlineMarkdown("run `pnpm install` first")).toBe("run pnpm install first");
  });

  it("strips strikethrough markers", () => {
    expect(stripInlineMarkdown("~~old~~ new plan")).toBe("old new plan");
  });

  it("replaces links with their text", () => {
    expect(stripInlineMarkdown("see [the docs](https://example.com)")).toBe("see the docs");
  });

  it("replaces images with their alt text", () => {
    expect(stripInlineMarkdown("logo ![alt text](https://example.com/a.png) here")).toBe(
      "logo alt text here",
    );
  });

  it("handles mixed inline formatting", () => {
    expect(stripInlineMarkdown("**Bold** and _italic_ with `code` in [link](https://a.b)")).toBe(
      "Bold and italic with code in link",
    );
  });

  it("strips emphasis nested inside a link", () => {
    expect(stripInlineMarkdown("[**bold link**](https://example.com)")).toBe("bold link");
  });

  it("keeps snake_case identifiers intact", () => {
    expect(stripInlineMarkdown("parse_markdown_to_blocks 함수")).toBe(
      "parse_markdown_to_blocks 함수",
    );
  });

  it("trims surrounding whitespace", () => {
    expect(stripInlineMarkdown("  spaced out  ")).toBe("spaced out");
  });

  it("returns empty string for whitespace-only input", () => {
    expect(stripInlineMarkdown("   ")).toBe("");
  });
});
