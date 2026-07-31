import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const capability = JSON.parse(
  readFileSync(
    new URL("../../../src-tauri/capabilities/default.json", import.meta.url),
    "utf8",
  ),
) as { windows?: string[]; permissions?: string[] };

describe("appearance preferences window capability", () => {
  it("allows dynamically created session windows to invoke and listen", () => {
    expect(capability.windows).toContain("session-*");
    expect(capability.permissions).toContain("core:default");
  });

  it("allows session windows to update their native title", () => {
    expect(capability.permissions).toContain("core:window:allow-set-title");
  });
});
