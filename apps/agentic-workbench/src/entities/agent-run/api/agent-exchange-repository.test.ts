import { readFileSync } from "node:fs";

const COMPAT_TRANSPORT_SOURCE = readFileSync(
  new URL("../../../shared/api/transport/compat-transport.ts", import.meta.url),
  "utf8",
);
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
    // 043: 창 삽입(`<event>-fallback`) 수신은 호환 transport가 맡고, 저장소는 창 transport의 listen을 쓴다.
    expect(SOURCE).toContain("listen(");
    expect(COMPAT_TRANSPORT_SOURCE).toContain("fallback: `${event}-fallback`");
  });
});
