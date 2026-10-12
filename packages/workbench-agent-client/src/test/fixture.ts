import { FakeEventHub } from "@yoophi/workbench-client/testing";
import type { RunEvent } from "@yoophi/workbench-client";
import { createAgentChatClient } from "../client";

export class Fixture {
  hub = new FakeEventHub();
  calls: Array<{
    operation: string;
    input: Record<string, unknown>;
    idempotencyKey?: string;
  }> = [];
  effects: Array<{ operation: string; input: Record<string, unknown> }> = [];
  journal = new Map<string, Array<{ sequence: number; event: RunEvent }>>();
  ledger = new Map<string, unknown>();
  lostReplies = 0;
  lostOperation = "run.sendPrompt";
  loseCreateBeforeApplying = false;
  restartOnLoss = false;
  replayFailures = 0;
  handshakesDown = false;
  replayWindow = Infinity;
  bench = 0;
  fetch = (async (url: string | URL | Request, init?: RequestInit) => {
    if (String(url).endsWith("/v1/event-tickets"))
      return this.hub.fetch(url, init);
    if (String(url).endsWith("/v1/system/handshake") && this.handshakesDown)
      throw new TypeError("server offline");
    if (String(url).endsWith("/v1/system/handshake"))
      return this.json({
        serverEpoch: this.hub.epoch,
        instanceId: "fixture",
        selectedProtocolVersion: 1,
        contractHash: "h",
        storageSchemaVersion: 2,
        features: [],
      });
    if (!String(url).endsWith("/v1/calls"))
      throw new Error(`unexpected URL ${url}`);
    const body = JSON.parse(String(init?.body));
    this.calls.push(body);
    const { operation, input, idempotencyKey } = body;
    if (operation === "agent.list")
      return this.json({
        kind: "complete",
        operation,
        output: [
          {
            id: "fake",
            name: "Fixture Agent",
            command: "fake-acp",
            models: [],
            permissionModes: [],
            effortOptions: [],
            contextSizes: [],
          },
        ],
      });
    if (
      operation === "run.createSession" &&
      this.loseCreateBeforeApplying &&
      this.lostReplies-- > 0
    ) {
      throw new TypeError("creation response unavailable");
    }
    if (operation === "run.replay") {
      if (this.replayFailures-- > 0)
        return this.json(
          {
            code: "unavailable",
            message: "snapshot temporarily unavailable",
            outcome: "notApplied",
            requestId: body.requestId,
            retryable: true,
          },
          503,
        );
      const journal = this.journal.get(input.runId) ?? [];
      const events = journal.slice(-this.replayWindow);
      return this.json({
        kind: "complete",
        operation,
        output: {
          runId: input.runId,
          events,
          lastSequence: journal[journal.length - 1]?.sequence ?? 0,
          terminal: journal.some(
            (e) =>
              e.event.type === "lifecycle" &&
              ["cancelled", "completed"].includes(e.event.status),
          ),
          gapDetected: events.length < journal.length,
        },
      });
    }
    let reply = this.ledger.get(idempotencyKey);
    if (!reply) {
      this.effects.push({ operation, input });
      let output: unknown = null;
      switch (operation) {
        case "bench.open":
          output = { benchId: `b-${++this.bench}` };
          break;
        case "run.createSession":
          output = {
            id: input.request.runId,
            agentId: input.request.agentId,
            goal: input.request.goal,
          };
          this.emit(input.request.runId, {
            type: "lifecycle",
            status: "sessionCreated",
            message: "ready",
          });
          break;
        case "run.sendPrompt":
          this.emit(input.runId, {
            type: "lifecycle",
            status: "promptSent",
            message: "sent",
          });
          this.emit(input.runId, { type: "agentMessage", text: "reply:" });
          this.emit(input.runId, { type: "agentMessage", text: input.prompt });
          this.emit(input.runId, {
            type: "lifecycle",
            status: "promptCompleted",
            message: "done",
          });
          break;
        case "run.cancel":
          this.emit(input.runId, {
            type: "lifecycle",
            status: "cancelled",
            message: "cancelled",
          });
          break;
        case "run.respondPermission":
          break;
        default:
          throw new Error(`unexpected operation ${operation}`);
      }
      reply = { kind: "complete", operation, output };
      this.ledger.set(idempotencyKey, reply);
    }
    if (operation === this.lostOperation && this.lostReplies-- > 0) {
      if (this.restartOnLoss) {
        this.hub.restart("epoch-2");
        this.journal.clear();
      }
      throw new TypeError("response lost");
    }
    return this.json(reply);
  }) as typeof fetch;
  json(body: unknown, status = 200) {
    return new Response(JSON.stringify(body), {
      status,
      headers: {
        "content-type":
          status === 200 ? "application/json" : "application/problem+json",
      },
    });
  }
  emit(id: string, event: RunEvent) {
    const envelope = this.hub.publish(`run:${id}`, event, "run.event.v1");
    this.journal.set(id, [
      ...(this.journal.get(id) ?? []),
      { sequence: envelope.sequence, event },
    ]);
    return envelope;
  }
  createClient() {
    return createAgentChatClient({
      workspace: "/fixture",
      connectionProvider: async () => ({
        baseUrl: "http://127.0.0.1:9",
        token: "fixture-token",
        expiresAt: new Date(Date.now() + 3600_000).toISOString(),
      }),
      fetch: this.fetch,
      openSocket: this.hub.openSocket,
    });
  }
}
