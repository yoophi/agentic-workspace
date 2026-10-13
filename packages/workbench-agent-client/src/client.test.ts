import { afterEach, describe, expect, it, vi } from "vitest";
import { until } from "@yoophi/workbench-client/testing";
import type { AgentChatClient } from "./types";
import { createAgentChatClient } from "./client";
import { Fixture } from "./test/fixture";

const clients: AgentChatClient[] = [];
afterEach(() => {
  for (const client of clients.splice(0)) client.dispose();
  vi.useRealTimers();
});
async function ready(fixture = new Fixture(), runId = "r1") {
  const client = fixture.createClient();
  clients.push(client);
  const opened = await client.connect();
  expect(opened.kind).toBe("ok");
  await client.createSession({ agentId: "fake", runId });
  await until(
    () => client.getSnapshot().sessions[0]?.status === "ready",
    "ready session",
  );
  return { fixture, client };
}

describe("shared agent chat Interface", () => {
  it("refreshes HTTP identity after a WS-only epoch change before opening the next workspace bench", async () => {
    const { fixture, client } = await ready();
    fixture.hub.restart("epoch-2");
    fixture.journal.clear();
    await until(() => client.getSnapshot().sessions[0].status === "lost");
    expect((await client.connect()).kind).toBe("ok");
    expect(client.getSnapshot().benchId).toBe("b-2");
    await client.createSession({ agentId: "fake", runId: "r2" });
    await until(() => client.getSnapshot().sessions[1].status === "ready");
    expect(client.getSnapshot().sessions[0].status).toBe("lost");
    expect(
      fixture.effects.some((call) => call.operation === "run.sendPrompt"),
    ).toBe(false);
  });
  it("discovers agents through the existing HTTP query without native transport or a mutation", async () => {
    const fixture = new Fixture();
    const client = fixture.createClient();
    clients.push(client);
    await client.connect();
    const agents = await client.listAgents();
    expect(agents.kind === "ok" && agents.output[0].id).toBe("fake");
    const query = fixture.calls.find(
      (call) => call.operation === "agent.list",
    )!;
    expect(query.idempotencyKey).toBeUndefined();
    expect(fixture.effects.map((call) => call.operation)).toEqual([
      "bench.open",
    ]);
  });
  it("preserves NotApplied while offline and does not enqueue a user prompt", async () => {
    vi.useFakeTimers();
    const { fixture, client } = await ready();
    fixture.handshakesDown = true;
    fixture.hub.ticketsDown = true;
    fixture.hub.sockets[fixture.hub.sockets.length - 1].drop();
    await until(() => client.getSnapshot().connection === "reconnecting");
    expect(await client.send("r1", "offline draft")).toEqual({
      kind: "notApplied",
      reason: "offline",
    });
    expect(client.getSnapshot().sessions[0].messages[0].delivery).toEqual({
      kind: "notApplied",
      reason: "offline",
    });
    expect(
      fixture.effects.some((call) => call.operation === "run.sendPrompt"),
    ).toBe(false);
  });
  it("does not claim cancellation when an uncertain creation has no server-owned evidence", async () => {
    vi.useFakeTimers();
    const fixture = new Fixture();
    const client = fixture.createClient();
    clients.push(client);
    await client.connect();
    fixture.loseCreateBeforeApplying = true;
    fixture.lostReplies = 2;
    expect(
      (await client.createSession({ agentId: "fake", runId: "r1" })).kind,
    ).toBe("unknown");
    const closing = client.closeSession("r1");
    await vi.advanceTimersByTimeAsync(10_100);
    expect(await closing).toEqual({ kind: "unknown", reason: "lost" });
    expect(client.getSnapshot().sessions[0].status).toBe("uncertain");
    expect(
      fixture.effects.some((call) => call.operation === "run.cancel"),
    ).toBe(false);
  });
  it("confirms an uncertain idle creation through read-only replay without creating another process", async () => {
    const fixture = new Fixture();
    const client = fixture.createClient();
    clients.push(client);
    await client.connect();
    fixture.lostOperation = "run.createSession";
    fixture.lostReplies = 2;
    expect(
      await client.createSession({ agentId: "fake", runId: "r1" }),
    ).toEqual({ kind: "unknown", reason: "lost" });
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    expect(client.getSnapshot().sessions[0].creationOutcome).toEqual({
      kind: "unknown",
      reason: "lost",
    });
    expect(
      fixture.effects.filter((call) => call.operation === "run.createSession"),
    ).toHaveLength(1);
    expect(
      fixture.calls.filter((call) => call.operation === "run.createSession"),
    ).toHaveLength(2);
    expect(
      fixture.effects.some((call) => call.operation === "run.sendPrompt"),
    ).toBe(false);
  });

  it("waits for creation admission before closing and coalesces concurrent close actions", async () => {
    const fixture = new Fixture();
    const client = fixture.createClient();
    clients.push(client);
    await client.connect();
    const creating = client.createSession({ agentId: "fake", runId: "r1" });
    const closing = client.closeSession("r1");
    const sameClose = client.closeSession("r1");
    expect(sameClose).toBe(closing);
    await Promise.all([creating, closing]);
    expect(fixture.effects.map((call) => call.operation)).toEqual([
      "bench.open",
      "run.createSession",
      "run.cancel",
    ]);
    expect(client.getSnapshot().sessions[0].status).toBe("closed");
  });
  it("serializes concurrent connect, creates an idle session and reduces two user turns once", async () => {
    const fixture = new Fixture();
    const client = fixture.createClient();
    clients.push(client);
    await Promise.all([client.connect(), client.connect()]);
    expect(
      fixture.effects.filter((call) => call.operation === "bench.open"),
    ).toHaveLength(1);
    const created = await client.createSession({
      agentId: "fake",
      runId: "r1",
      label: "Metadata",
    });
    expect(created.kind).toBe("ok");
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    expect(fixture.effects.map((call) => call.operation)).toEqual([
      "bench.open",
      "run.createSession",
    ]);
    await client.send("r1", "one");
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    await client.send("r1", "two");
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    expect(
      client.getSnapshot().sessions[0].messages.map((message) => message.text),
    ).toEqual(["one", "reply:one", "two", "reply:two"]);
    expect(
      client
        .getSnapshot()
        .sessions[0].messages.filter((message) => message.role === "user")
        .every((message) => message.delivery?.kind === "ok"),
    ).toBe(true);
    expect(JSON.stringify(client.getSnapshot())).not.toContain("fixture-token");
  });

  it("does not lose conversation or cancel a run when UI listeners hide or dispose", async () => {
    const { fixture, client } = await ready();
    const hidden = client.subscribe(() => undefined);
    hidden();
    const bad = client.subscribe(() => {
      throw new Error("UI render failed");
    });
    await client.send("r1", "while-hidden");
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    bad();
    let shown: string[] = [];
    client.subscribe((state) => {
      shown = state.sessions[0].messages.map((message) => message.text);
    });
    expect(shown).toEqual(["while-hidden", "reply:while-hidden"]);
    client.dispose();
    client.dispose();
    expect(
      fixture.effects.some((call) =>
        ["run.cancel", "bench.close"].includes(call.operation),
      ),
    ).toBe(false);
    expect(fixture.hub.sockets.every((socket) => socket.readyState === 3)).toBe(
      true,
    );
    expect(client.getSnapshot().disposed).toBe(true);
  });

  it("reuses the transport's idempotency key after losing an applied response", async () => {
    const { fixture, client } = await ready();
    fixture.lostReplies = 1;
    expect((await client.send("r1", "only-once")).kind).toBe("ok");
    await until(() => client.getSnapshot().sessions[0].status === "ready");
    const attempts = fixture.calls.filter(
      (call) => call.operation === "run.sendPrompt",
    );
    expect(attempts).toHaveLength(2);
    expect(attempts[0].idempotencyKey).toBe(attempts[1].idempotencyKey);
    expect(
      fixture.effects.filter((call) => call.operation === "run.sendPrompt"),
    ).toHaveLength(1);
    expect(
      client.getSnapshot().sessions[0].messages.map((message) => message.text),
    ).toEqual(["only-once", "reply:only-once"]);
  });

  it("preserves an Unknown result and does not make a third send attempt", async () => {
    const { fixture, client } = await ready();
    fixture.lostReplies = 2;
    expect(await client.send("r1", "uncertain")).toEqual({
      kind: "unknown",
      reason: "lost",
    });
    expect(client.getSnapshot().sessions[0].status).toBe("uncertain");
    await expect(client.send("r1", "uncertain")).rejects.toMatchObject({
      code: "sessionNotReady",
    });
    expect(
      fixture.calls.filter((call) => call.operation === "run.sendPrompt"),
    ).toHaveLength(2);
    expect(
      fixture.effects.filter((call) => call.operation === "run.sendPrompt"),
    ).toHaveLength(1);
    expect(client.getSnapshot().sessions[0].messages[0].delivery).toEqual({
      kind: "unknown",
      reason: "lost",
    });
  });

  it("keeps the old transcript as lost across an epoch change and never retries its prompt on the new server", async () => {
    const { fixture, client } = await ready();
    fixture.lostReplies = 1;
    fixture.restartOnLoss = true;
    expect(await client.send("r1", "old-server")).toEqual({
      kind: "unknown",
      reason: "epochChanged",
    });
    expect(client.getSnapshot().benchId).toBeUndefined();
    expect(client.getSnapshot().sessions[0].status).toBe("lost");
    expect(
      fixture.calls.filter((call) => call.operation === "run.sendPrompt"),
    ).toHaveLength(1);
    await client.connect();
    expect(client.getSnapshot().benchId).toBe("b-2");
    await expect(client.send("r1", "retry")).rejects.toMatchObject({
      code: "sessionNotReady",
    });
    expect(client.getSnapshot().sessions[0].messages[0].text).toBe(
      "old-server",
    );
    await client.closeSession("r1");
    expect(client.getSnapshot().sessions[0].status).toBe("closed");
    expect(
      fixture.effects.some((call) => call.operation === "run.cancel"),
    ).toBe(false);
  });

  it("validates permission request and option against the selected session", async () => {
    const { fixture, client } = await ready();
    await client.createSession({ agentId: "fake", runId: "r2" });
    await until(() => client.getSnapshot().sessions[1].status === "ready");
    fixture.emit("r1", {
      type: "permission",
      permissionId: "p1",
      title: "Allow?",
      options: [{ optionId: "allow", name: "Allow", kind: "allow_once" }],
      requiresResponse: true,
    });
    await until(
      () => client.getSnapshot().sessions[0].permissions.length === 1,
    );
    await expect(
      client.respondPermission("r2", "p1", "allow"),
    ).rejects.toMatchObject({ code: "invalidPermission" });
    await expect(
      client.respondPermission("r1", "p1", "unknown"),
    ).rejects.toMatchObject({ code: "invalidPermission" });
    expect((await client.respondPermission("r1", "p1", "allow")).kind).toBe(
      "ok",
    );
    expect(client.getSnapshot().sessions[0].permissions).toEqual([]);
    expect(
      fixture.effects
        .filter((call) => call.operation === "run.respondPermission")
        .map((call) => call.input),
    ).toEqual([
      { benchId: "b-1", runId: "r1", permissionId: "p1", optionId: "allow" },
    ]);
  });

  it("ignores duplicated event frames and only explicit close cancels the selected session", async () => {
    const { fixture, client } = await ready();
    const frame = fixture.emit("r1", { type: "agentMessage", text: "chunk" });
    await until(() => client.getSnapshot().sessions[0].messages.length === 1);
    fixture.hub.sockets[fixture.hub.sockets.length - 1]?.onmessage?.({
      data: JSON.stringify({ type: "event", event: frame }),
    });
    await until(
      () => client.getSnapshot().sessions[0].lastSequence === frame.sequence,
    );
    expect(
      client.getSnapshot().sessions[0].messages.map((message) => message.text),
    ).toEqual(["chunk"]);
    expect((await client.closeSession("r1")).kind).toBe("ok");
    expect(client.getSnapshot().sessions[0].status).toBe("closed");
    await client.closeSession("r1");
    expect(
      fixture.effects.filter((call) => call.operation === "run.cancel"),
    ).toHaveLength(1);
  });

  it("ends a pending bootstrap on disposal without opening a bench after credentials arrive", async () => {
    const fixture = new Fixture();
    let provide!: (value: {
      baseUrl: string;
      token: string;
      expiresAt: string;
    }) => void;
    const client = createAgentChatClient({
      workspace: "/fixture",
      fetch: fixture.fetch,
      connectionProvider: () =>
        new Promise((resolve) => {
          provide = resolve;
        }),
    });
    clients.push(client);
    const connecting = client.connect();
    client.dispose();
    await expect(connecting).rejects.toMatchObject({ code: "disposed" });
    provide({
      baseUrl: "http://127.0.0.1:9",
      token: "fixture-token",
      expiresAt: new Date(Date.now() + 3600_000).toISOString(),
    });
    await until(() => client.getSnapshot().disposed);
    expect(fixture.effects).toEqual([]);
  });

  it("recovers a retention gap through replay, keeps visible history and marks missing content", async () => {
    vi.useFakeTimers();
    const { fixture, client } = await ready();
    fixture.emit("r1", { type: "agentMessage", text: "visible" });
    await until(() => client.getSnapshot().sessions[0].messages.length === 1);
    fixture.hub.sockets[fixture.hub.sockets.length - 1]!.drop();
    fixture.hub.capacity = 2;
    fixture.replayWindow = 2;
    for (const text of ["missed-1", "missed-2", "retained-1", "retained-2"])
      fixture.emit("r1", { type: "agentMessage", text });
    await vi.advanceTimersByTimeAsync(500);
    await until(
      () =>
        client.getSnapshot().sessions[0].lastSequence ===
        fixture.hub.lastSequence("run:r1"),
    );
    const session = client.getSnapshot().sessions[0];
    expect(session.historyIncomplete).toBe(true);
    expect(session.messages.map((message) => message.text)).toEqual([
      "visible",
      "retained-1retained-2",
    ]);
    expect(session.status).toBe("ready");
  });

  it("retries a failed replay snapshot without duplicating already applied chunks", async () => {
    vi.useFakeTimers();
    const { fixture, client } = await ready();
    fixture.hub.sockets[fixture.hub.sockets.length - 1]!.drop();
    fixture.hub.capacity = 1;
    fixture.emit("r1", { type: "agentMessage", text: "first" });
    fixture.emit("r1", { type: "agentMessage", text: "second" });
    fixture.replayFailures = 1;
    await vi.advanceTimersByTimeAsync(2000);
    await until(() => client.getSnapshot().sessions[0].lastSequence === 3);
    expect(
      client.getSnapshot().sessions[0].messages.map((message) => message.text),
    ).toEqual(["firstsecond"]);
    expect(client.getSnapshot().sessions[0].status).toBe("ready");
  });
});
