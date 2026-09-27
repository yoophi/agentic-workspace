// 호출 클라이언트(043 T023, research R6, contracts §4). 결과는 넷이다:
// - ok / fault: 서버가 답했다(fault는 problem 응답).
// - notApplied: 연결이 이미 끊겨 보내지 않았다.
// - unknown: 보내기를 시도한 뒤 답을 받지 못했다. 변경은 재연결 세대가 같을 때만 같은 멱등성 키로 한 번 더 보낸다 —
//   세대가 바뀌었으면(서버 재기동, 세대 범위 멱등 기록이 사라짐) 다시 보내지 않는다. 조회는 새 요청으로 다시 보낸다.
import type { Handshake } from "./connection";
import { isQueryOperation } from "./operation-kinds";
import type { OperationId, OperationMap, WorkbenchFault } from "./operation-map";
import { CALLS_PATH, PROTOCOL_VERSION } from "./operation-map";

export interface ConnectionPort {
  state(): "connecting" | "connected" | "reconnecting" | "disconnected";
  epoch(): string | undefined;
  credentials(): { baseUrl: string; token: string };
  refreshCredentials(): Promise<void>;
  reportLost(): void;
  whenConnected(): Promise<Pick<Handshake, "serverEpoch">>;
}

export type CallOutcome<Output> =
  | { kind: "ok"; output: Output; revision: number | undefined }
  | { kind: "fault"; fault: WorkbenchFault }
  | { kind: "notApplied"; reason: "offline" }
  | { kind: "unknown"; reason: "epochChanged" | "lost" };

export interface CallOptions {
  /** 사용자 조작 하나당 하나. 없으면 변경마다 새로 만든다. */
  idempotencyKey?: string;
}

export interface WorkbenchClient {
  call<K extends OperationId>(
    operation: K,
    input: OperationMap[K]["input"],
    options?: CallOptions,
  ): Promise<CallOutcome<OperationMap[K]["output"]>>;
}

export interface WorkbenchClientOptions {
  connection: ConnectionPort;
  fetch?: typeof fetch;
}

type Sent =
  | { kind: "reply"; response: Response; body: unknown }
  | { kind: "lost" };

function randomId() {
  return crypto.randomUUID();
}

export function createWorkbenchClient(options: WorkbenchClientOptions): WorkbenchClient {
  const fetchImpl = options.fetch ?? fetch;
  const { connection } = options;

  async function send(body: Record<string, unknown>): Promise<Sent> {
    const { baseUrl, token } = connection.credentials();
    try {
      const response = await fetchImpl(`${baseUrl}${CALLS_PATH}`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const parsed: unknown = await response.json();
      const isProblem = (response.headers.get("content-type") ?? "").includes("problem+json");
      const isReply = typeof parsed === "object" && parsed !== null && ("kind" in parsed || isProblem);
      if (!isReply) {
        return { kind: "lost" };
      }
      return { kind: "reply", response, body: parsed };
    } catch {
      return { kind: "lost" };
    }
  }

  function outcomeOf<Output>(sent: Extract<Sent, { kind: "reply" }>): CallOutcome<Output> {
    if (sent.response.ok) {
      const reply = sent.body as { output?: Output; revision?: number | null };
      return { kind: "ok", output: reply.output as Output, revision: reply.revision ?? undefined };
    }
    return { kind: "fault", fault: sent.body as WorkbenchFault };
  }

  /** 한 번 보낸다. 401이면 자격 증명을 갱신해 한 번 더 — 401 자체는 적용 전 거절이지만, 앞선 시도가 불확실했던 변경
   *  (`boundEpoch`가 있음)은 갱신으로 끝점·세대가 바뀌었으면 다시 보내지 않는다(세대 범위 멱등 기록이 새 서버에 없다). */
  async function sendOnce<Output>(
    body: Record<string, unknown>,
    boundEpoch: string | undefined,
  ): Promise<CallOutcome<Output> | { kind: "lost" } | { kind: "epochChanged" }> {
    let sent = await send(body);
    if (sent.kind === "reply" && sent.response.status === 401) {
      try {
        await connection.refreshCredentials();
      } catch {
        return { kind: "lost" };
      }
      if (boundEpoch !== undefined && connection.epoch() !== boundEpoch) {
        return { kind: "epochChanged" };
      }
      sent = await send(body);
    }
    if (sent.kind === "lost") {
      return sent;
    }
    return outcomeOf<Output>(sent);
  }

  return {
    async call(operation, input, callOptions = {}) {
      if (connection.state() !== "connected") {
        return { kind: "notApplied", reason: "offline" };
      }
      const query = isQueryOperation(operation);
      const sentEpoch = connection.epoch();
      const idempotencyKey = query ? undefined : (callOptions.idempotencyKey ?? randomId());
      const envelope = () => ({
        protocolVersion: PROTOCOL_VERSION,
        operation,
        requestId: randomId(),
        input,
        ...(idempotencyKey === undefined ? {} : { idempotencyKey }),
      });
      const first = await sendOnce(envelope(), undefined);
      if (first.kind !== "lost" && first.kind !== "epochChanged") {
        return first;
      }
      if (first.kind === "lost") {
        connection.reportLost();
      }
      const handshake = await connection.whenConnected();
      // 이 뒤의 변경 재시도는 모두 처음 보낸 세대에 묶인다.
      const bound = query ? undefined : sentEpoch;
      if (bound !== undefined && handshake.serverEpoch !== bound) {
        return { kind: "unknown", reason: "epochChanged" };
      }
      const retried = await sendOnce(envelope(), bound);
      if (retried.kind === "epochChanged") {
        return { kind: "unknown", reason: "epochChanged" };
      }
      if (retried.kind === "lost") {
        connection.reportLost();
        return { kind: "unknown", reason: "lost" };
      }
      return retried;
    },
  } as WorkbenchClient;
}
