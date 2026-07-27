import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(new URL("./agent-exchange-repository.ts", import.meta.url), "utf8");

describe("agent exchange repository", () => {
  it("maps workspace and exchange contracts to Tauri commands and events", () => {
    for (const command of [
      "sync_agent_workspace",
      "send_agent_exchange",
      "acknowledge_agent_exchange",
      "list_agent_exchanges",
    ]) {
      expect(SOURCE).toContain(`"${command}"`);
    }
    expect(SOURCE).toContain('"agent-exchange-requested"');
    expect(SOURCE).toContain('"agent-exchange-status"');
    expect(SOURCE).toContain("fallback");
  });
});
