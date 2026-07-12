import { describe, expect, it } from "vitest";
import { stripHtmlComments, transformWikilinks } from "./inline-markdown";

describe("transformWikilinks", () => {
  it("converts plain and aliased wikilinks to relative markdown links", () => {
    expect(transformWikilinks("[[next]] and [[plan | 구현 계획]] and [[ready.md]]")).toBe(
      "[next](./next.md) and [구현 계획](./plan.md) and [ready.md](./ready.md)",
    );
  });

  it("encodes spaces and Korean characters in targets without changing the label", () => {
    expect(transformWikilinks("[[링크 문서 | 연결 문서]]")).toBe(
      "[연결 문서](./%EB%A7%81%ED%81%AC%20%EB%AC%B8%EC%84%9C.md)",
    );
  });

  it("preserves malformed links and inline code", () => {
    expect(transformWikilinks("[[]] [[ | label]] `[[code]]`")).toBe(
      "[[]] [[ | label]] `[[code]]`",
    );
  });
});

describe("stripHtmlComments", () => {
  it("hides complete single-line and multiline comments while retaining lines", () => {
    expect(stripHtmlComments("before <!-- secret --> after\n<!-- multi\nsecret -->\nend")).toBe(
      "before  after\n\n\nend",
    );
  });

  it("preserves comment-like text in fenced and inline code", () => {
    const markdown = "`<!-- inline -->`\n```html\n<!-- fenced -->\n```";
    expect(stripHtmlComments(markdown)).toBe(markdown);
  });

  it("preserves an unclosed comment rather than swallowing later content", () => {
    const markdown = "before\n<!-- unclosed\nafter";
    expect(stripHtmlComments(markdown)).toBe(markdown);
  });
});
