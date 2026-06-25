import { describe, expect, it } from "vitest";

import { isOpenableExternalUrl } from "./external-url";

describe("isOpenableExternalUrl", () => {
  it("allows http and https URLs", () => {
    expect(isOpenableExternalUrl("https://example.com/docs")).toBe(true);
    expect(isOpenableExternalUrl("http://localhost:1420")).toBe(true);
  });

  it("rejects relative URLs and unsafe schemes", () => {
    expect(isOpenableExternalUrl("/relative/path")).toBe(false);
    expect(isOpenableExternalUrl("javascript:alert(1)")).toBe(false);
    expect(isOpenableExternalUrl("file:///tmp/readme.md")).toBe(false);
    expect(isOpenableExternalUrl(undefined)).toBe(false);
  });
});
