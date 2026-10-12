import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, expect, it, vi } from "vitest";
import {
  createConnection,
  createWorkbenchClient,
  type SocketLike,
} from "@yoophi/workbench-client";
import { startHost } from "@yoophi/workbench-client/test-host";
import { createAgentChatClient } from "./client";

const cleanup: Array<() => void | Promise<void>> = [];
afterEach(async () => {
  for (const step of cleanup.splice(0).reverse()) await step();
});

it("uses production HTTP/WS with an idle ACP process, two permission-gated turns, reconnect and detach", async () => {
  const host = await startHost({
    HOST_USE_ACP_ENGINE: "1",
    HOST_JOURNAL_CAPACITY: "1000",
  });
  cleanup.push(() => host.stop());
  const sockets: WebSocket[] = [];
  const provider = async () => ({
    baseUrl: host.baseUrl,
    token: host.tokens.windowA,
    expiresAt: new Date(Date.now() + 3600_000).toISOString(),
    incarnation: "inc-1",
  });
  const client = createAgentChatClient({
    workspace: host.workDir,
    connectionProvider: provider,
    openSocket: (url) => {
      const socket = new WebSocket(url);
      sockets.push(socket);
      return socket as unknown as SocketLike;
    },
  });
  cleanup.push(() => client.dispose());
  expect((await client.connect()).kind).toBe("ok");
  const script = resolve(
    __dirname,
    "../../../crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py",
  );
  const log = resolve(host.dataDir, "agent.log");
  const pidFile = resolve(host.dataDir, "pid");
  expect(
    (
      await client.createSession({
        agentId: "fake-acp",
        runId: "r1",
        label: "session metadata",
        agentCommand: `python3 ${script} --log ${log} --log-session --pid-file ${pidFile} --echo`,
        permissionMode: "default",
      })
    ).kind,
  ).toBe("ok");
  await vi.waitFor(
    () => expect(client.getSnapshot().sessions[0].status).toBe("ready"),
    { timeout: 15_000 },
  );
  expect(readFileSync(log, "utf8").trim().split("\n")).toEqual([
    "initialize",
    "session/new",
  ]);
  const pid = Number(readFileSync(pidFile, "utf8"));
  const session = () => client.getSnapshot().sessions[0];
  const watch = client.subscribe(() => undefined);
  watch();
  for (const text of ["one", "two", "after-reconnect"]) {
    if (text === "after-reconnect") sockets[sockets.length - 1].close();
    expect((await client.send("r1", text)).kind).toBe("ok");
    await vi.waitFor(() => expect(session().permissions).toHaveLength(1), {
      timeout: 10_000,
    });
    const permission = session().permissions[0];
    expect(
      (await client.respondPermission("r1", permission.id, "allow")).kind,
    ).toBe("ok");
    await vi.waitFor(() => expect(session().status).toBe("ready"), {
      timeout: 10_000,
    });
  }
  expect(session().messages.map((message) => message.text)).toEqual([
    "one",
    "echo:one",
    "two",
    "echo:two",
    "after-reconnect",
    "echo:after-reconnect",
  ]);
  expect(sockets.length).toBeGreaterThan(1);
  expect(
    readFileSync(log, "utf8")
      .split("\n")
      .filter((line) => line.startsWith("prompt:")),
  ).toHaveLength(3);
  expect(readFileSync(log, "utf8")).not.toContain("session metadata");
  const bench = client.getSnapshot().benchId!;
  client.dispose();
  expect(() => process.kill(pid, 0)).not.toThrow(); // UI disposal does not cancel a server-owned session.
  const owner = createConnection({ fetchConnection: provider });
  cleanup.push(() => owner.close());
  await owner.start();
  const calls = createWorkbenchClient({ connection: owner });
  expect(
    (await calls.call("run.cancel", { benchId: bench, runId: "r1" })).kind,
  ).toBe("ok");
  await vi.waitFor(() => expect(() => process.kill(pid, 0)).toThrow(), {
    timeout: 5000,
  });
});
