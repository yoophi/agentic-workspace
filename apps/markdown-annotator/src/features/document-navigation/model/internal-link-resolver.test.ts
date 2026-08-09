import { describe, expect, it } from "vitest";
import { resolveInternalLink } from "./internal-link-resolver";

describe("resolveInternalLink", () => {
  it("resolves same-root Markdown links and headings", () => {
    expect(resolveInternalLink("docs/current.md", "../plan.markdown#Next")).toEqual({ kind: "document", relativePath: "plan.markdown", heading: "Next" });
    expect(resolveInternalLink("docs/current.md", "#개요")).toEqual({ kind: "heading", heading: "개요" });
  });
  it("allows only HTTP and HTTPS external links", () => {
    expect(resolveInternalLink("a.md", "https://example.com/x").kind).toBe("external");
    expect(() => resolveInternalLink("a.md", "file:///tmp/a.md")).toThrow();
  });
  it.each(["../../secret.md", "/secret.md", "note.txt", "..\\secret.md"])("rejects unsafe target %s", (href) => expect(() => resolveInternalLink("docs/current.md", href)).toThrow());
});
