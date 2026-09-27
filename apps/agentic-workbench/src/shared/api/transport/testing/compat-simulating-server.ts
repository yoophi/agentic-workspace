// 시험 전용(043 T027): 실제 루프백 HTTP로 뜨는 가짜 Workbench 서버. `/v1/system/handshake`와 `/v1/calls`에 답하고, 받은
// operation 입력을 command 인자로 되돌려(`reverseArgs`) 화면 시험이 이미 쓰는 command 단위 가짜 응답(`invoke(command,
// args)`)을 부른다 — 같은 시나리오·같은 기대값을 네트워크 경로(실제 fetch → call client → HttpTransport)로 돌리기 위해서다.
// 응답은 서버 모양으로 되돌린다: orchestration 묶임 결과는 `eventStreamId`를 가진 core DTO, 교환·orchestration 오류는
// details가 붙은 problem. 역변환의 정확성은 golden 사례 왕복 시험(`compat-simulating-server.test.ts`)이 지킨다.
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";

import { createConnection, createWorkbenchClient, type Connection } from "@yoophi/workbench-client";

import { COMMANDS } from "../command-table";
import { createHttpTransport } from "../http-transport";
import type { Transport } from "../transport";

type Args = Record<string, unknown>;
type Input = Record<string, unknown>;

const commandOfOperation = new Map(Object.entries(COMMANDS).map(([command, spec]) => [spec.operation, command]));

function omitBench(input: Input): Args {
  const { benchId: _bench, ...rest } = input;
  return rest;
}

function defined(value: Args): Args {
  return Object.fromEntries(Object.entries(value).filter(([, item]) => item !== undefined));
}

/** operation 입력 → 화면이 command에 넘겼을 인자. */
export function reverseArgs(command: string, input: Input): Args {
  const rest = omitBench(input);
  switch (command) {
    case "create_project":
    case "create_saved_prompt":
    case "create_goal":
      return { input: rest };
    case "update_project":
    case "update_saved_prompt": {
      const { id, ...fields } = rest;
      return { id, input: fields };
    }
    case "update_goal":
    case "record_goal_progress": {
      const { workingDirectory, ...fields } = rest;
      return { workingDirectory, input: fields };
    }
    case "create_git_worktree": {
      const { workingDirectory, ...draft } = rest;
      return { workingDirectory, input: draft };
    }
    case "list_agent_tool_command_candidates":
      return { input: rest.query };
    case "set_run_permission_mode":
      return { runId: rest.runId, permissionMode: rest.mode };
    case "bootstrap_orchestration_workspace":
    case "list_recoverable_orchestration_workspaces":
    case "adopt_manual_orchestration_child":
    case "list_orchestration_tasks":
    case "replay_orchestration_runtime_events":
      return { input: rest };
    case "send_orchestration_child_command":
      return { input: rest.input };
    case "delegate_orchestration_goal":
    case "respond_orchestration_input":
    case "dispatch_orchestration_prompt":
    case "bind_main_coordinator_run":
    case "set_orchestration_presentation":
    case "cancel_orchestration_task":
    case "retry_orchestration_task":
    case "reassign_orchestration_task":
    case "handoff_orchestration_coordinator":
      return { input: rest.request };
    default:
      return defined(rest);
  }
}

/** compat 결과 → 서버가 돌려줬을 결과(HttpTransport가 다시 compat 모양으로 바꾼다). */
function serverOutput(command: string, output: unknown): unknown {
  const spec = COMMANDS[command];
  const bindsWindow = spec.output !== undefined;
  if (bindsWindow && output && typeof output === "object" && !Array.isArray(output)) {
    const { boundWindowLabel: _label, ...rest } = output as Args;
    return { ...rest, eventStreamId: "orchestration:test-binding" };
  }
  return output === undefined ? null : output;
}

/** compat 오류 문자열 → 서버 problem(HttpTransport가 같은 문자열을 다시 만든다). */
function problemOf(command: string, error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const flavor = COMMANDS[command]?.flavor;
  let details: unknown = null;
  let text = message;
  if (flavor === "exchange" || flavor === "orchestration") {
    try {
      const parsed = JSON.parse(message) as Args;
      if (flavor === "exchange" && typeof parsed.code === "string") {
        details = { exchangeCode: parsed.code };
        text = String(parsed.message);
      } else if (flavor === "orchestration") {
        details = { orchestrationError: parsed };
        text = String(parsed.message ?? message);
      }
    } catch {
      // 일반 문자열 오류
    }
  }
  return { code: "conflict", message: text, retryable: false, outcome: "notApplied", requestId: "fake", details, type: "urn:aw:fault:conflict", title: "conflict", status: 409 };
}

function readBody(request: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    request.on("error", reject);
  });
}

// 042 서버의 CORS(`CorsLayer`)와 같게: 요청 출처(시험 환경의 출처가 허용 목록에 있다고 본다)를 되돌리고, 자격 증명 쿠키는
// 허용하지 않으며, preflight는 POST·GET과 authorization·content-type을 허용한다.
function corsHeaders(request: IncomingMessage): Record<string, string> {
  const origin = request.headers.origin;
  return origin ? { "access-control-allow-origin": origin, vary: "origin" } : {};
}

function send(
  request: IncomingMessage,
  response: ServerResponse,
  status: number,
  body: unknown,
  contentType = "application/json",
) {
  response.writeHead(status, { "content-type": contentType, ...corsHeaders(request) });
  response.end(JSON.stringify(body));
}

export interface CompatSimulatingServer {
  baseUrl: string;
  transport: Transport;
  connection: Connection;
  close(): Promise<void>;
}

export async function startCompatSimulatingServer(
  invoke: (command: string, args?: Args) => unknown,
  options: { windowLabel?: string; benchId?: string } = {},
): Promise<CompatSimulatingServer> {
  const server = createServer(async (request, response) => {
    if (request.method === "OPTIONS") {
      response.writeHead(200, {
        ...corsHeaders(request),
        "access-control-allow-methods": "GET,POST",
        "access-control-allow-headers": "authorization,content-type",
      });
      response.end();
      return;
    }
    const body = await readBody(request);
    if (request.url === "/v1/system/handshake") {
      send(request, response, 200, {
        selectedProtocolVersion: 1, supportedProtocolVersions: [1], serverVersion: "fake", apiMajor: 1,
        contractHash: "fake", instanceId: "fake", serverEpoch: "fake-epoch", storageSchemaVersion: 2, features: [],
      });
      return;
    }
    if (request.url !== "/v1/calls") {
      send(request, response, 404, { message: "not found" });
      return;
    }
    const call = JSON.parse(body) as { operation: string; input: Input };
    const command = commandOfOperation.get(call.operation as never);
    if (!command) {
      send(request, response, 400, problemOf("", `unknown operation ${call.operation}`), "application/problem+json");
      return;
    }
    try {
      const output = await invoke(command, reverseArgs(command, call.input ?? {}));
      send(request, response, 200, { kind: "complete", output: serverOutput(command, output) });
    } catch (error) {
      send(request, response, 409, problemOf(command, error), "application/problem+json");
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("fake server has no port");
  }
  const baseUrl = `http://127.0.0.1:${address.port}`;
  const connection = createConnection({
    fetchConnection: async () => ({
      baseUrl,
      token: "fake-token",
      expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
      incarnation: "fake-incarnation",
    }),
  });
  await connection.start();
  const client = createWorkbenchClient({ connection });
  const benchId = options.benchId ?? "bench-test";
  const transport = createHttpTransport({
    client,
    ensureWindowBench: async () => benchId,
    windowLabel: options.windowLabel ?? "session-test",
  });
  return {
    baseUrl,
    transport,
    connection,
    close: async () => {
      connection.close();
      server.closeAllConnections?.();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    },
  };
}
