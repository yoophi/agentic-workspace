// 시험 전용(043 T029–T040): 042 event hub의 cursor 판정(`event_hub/stream.rs::decide_existing`·`decide_missing`)과 WS 프레임을
// 흉내 내는 가짜 hub. 표 발급(`POST /v1/event-tickets`)은 가짜 fetch로, 연결(`/v1/events?ticket=`)은 가짜 소켓으로 받는다.
// 전달은 시험이 `publish`를 부를 때 동기로 일어난다 — 시간 대기 없이 결정적으로 검사하기 위해서다.
import type { EventEnvelope, EventFrame, StreamCursor } from "../operation-map";

interface StreamLog {
  sequence: number;
  journal: EventEnvelope[];
  evicted: boolean;
}

export interface FakeSocket {
  url: string;
  readyState: number;
  onopen: ((event: unknown) => void) | null;
  onmessage: ((event: { data: string }) => void) | null;
  onclose: ((event: { code: number; reason: string }) => void) | null;
  onerror: ((event: unknown) => void) | null;
  close(): void;
  /** 가짜 hub가 소켓을 끊는다(연결 끊김 흉내). */
  drop(): void;
}

export class FakeEventHub {
  epoch = "epoch-1";
  capacity: number;
  private streams = new Map<string, StreamLog>();
  private tickets = new Map<string, StreamCursor[]>();
  private nextTicket = 0;
  readonly sockets: Array<FakeSocket & { cursors: StreamCursor[]; live: Set<string> }> = [];
  ticketRequests: StreamCursor[][] = [];
  /** 표 발급을 실패시킨다(서버 다운 흉내). */
  ticketsDown = false;

  constructor(capacity = 1_000_000) {
    this.capacity = capacity;
  }

  private log(streamId: string): StreamLog {
    let log = this.streams.get(streamId);
    if (!log) {
      log = { sequence: 0, journal: [], evicted: false };
      this.streams.set(streamId, log);
    }
    return log;
  }

  publish(streamId: string, body: unknown = {}, schema = "test.v1"): EventEnvelope {
    const log = this.log(streamId);
    log.sequence += 1;
    const event: EventEnvelope = {
      eventId: `${streamId}#${log.sequence}`,
      streamId,
      epoch: this.epoch,
      sequence: log.sequence,
      schema,
      occurredAt: "2026-09-28T00:00:00Z",
      body,
    };
    log.journal.push(event);
    while (log.journal.length > this.capacity) {
      log.journal.shift();
    }
    for (const socket of this.sockets) {
      if (socket.readyState === 1 && socket.live.has(streamId)) {
        socket.onmessage?.({ data: JSON.stringify({ type: "event", event } satisfies EventFrame) });
      }
    }
    return event;
  }

  evict(streamId: string) {
    this.log(streamId).evicted = true;
  }

  /** 서버 재기동: 새 세대, 스트림 초기화. */
  restart(epoch: string) {
    this.epoch = epoch;
    this.streams.clear();
    for (const socket of [...this.sockets]) {
      socket.drop();
    }
  }

  lastSequence(streamId: string) {
    return this.log(streamId).sequence;
  }

  fetch = (async (url: string | URL | Request, init?: RequestInit) => {
    const target = String(url);
    if (target.endsWith("/v1/event-tickets")) {
      if (this.ticketsDown) {
        throw new TypeError("fetch failed");
      }
      const body = JSON.parse(String(init?.body)) as { cursors: StreamCursor[] };
      this.ticketRequests.push(body.cursors);
      this.nextTicket += 1;
      const ticket = `ticket-${this.nextTicket}`;
      this.tickets.set(ticket, body.cursors);
      return new Response(JSON.stringify({ ticket, expiresAt: "2026-09-28T00:00:30Z" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    throw new Error(`fake hub: unexpected fetch ${target}`);
  }) as unknown as typeof fetch;

  openSocket = (url: string): FakeSocket => {
    const ticket = new URL(url).searchParams.get("ticket") ?? "";
    const cursors = this.tickets.get(ticket) ?? [];
    this.tickets.delete(ticket);
    const hub = this;
    const socket = {
      url,
      readyState: 0,
      onopen: null,
      onmessage: null,
      onclose: null,
      onerror: null,
      cursors,
      live: new Set<string>(),
      close() {
        if (socket.readyState === 3) {
          return;
        }
        socket.readyState = 3;
        queueMicrotask(() => socket.onclose?.({ code: 1000, reason: "client" }));
      },
      drop() {
        if (socket.readyState === 3) {
          return;
        }
        socket.readyState = 3;
        socket.onclose?.({ code: 1006, reason: "dropped" });
      },
    } as FakeSocket & { cursors: StreamCursor[]; live: Set<string> };
    this.sockets.push(socket);
    queueMicrotask(() => {
      if (socket.readyState === 3) {
        return;
      }
      socket.readyState = 1;
      socket.onopen?.({});
      const send = (frame: EventFrame) => socket.onmessage?.({ data: JSON.stringify(frame) });
      // 042: hello는 구독 등록 뒤에 간다. 판정 결과(replay·gap)는 그 뒤 프레임으로.
      const frames: EventFrame[] = [];
      for (const cursor of cursors) {
        const log = hub.log(cursor.streamId);
        const gap = (reason: "retentionExceeded" | "epochChanged" | "evicted", first?: number, last?: number) =>
          frames.push({ type: "gap", streamId: cursor.streamId, epoch: hub.epoch, reason, firstSequence: first, lastSequence: last });
        if (log.evicted) {
          gap("evicted");
          continue;
        }
        const first = log.journal[0]?.sequence ?? log.sequence + 1;
        if (cursor.afterSequence === 0) {
          if (first > 1) {
            gap("retentionExceeded", first, log.sequence);
            continue;
          }
        } else if (cursor.epoch !== hub.epoch) {
          gap("epochChanged");
          continue;
        } else if (cursor.afterSequence > log.sequence) {
          frames.push({ type: "fault", fault: { code: "invalidArgument", message: "cursor is ahead of the stream.", retryable: false, outcome: "notApplied", requestId: "fake" } });
          continue;
        } else if (cursor.afterSequence + 1 < first) {
          gap("retentionExceeded", first, log.sequence);
          continue;
        }
        socket.live.add(cursor.streamId);
        for (const event of log.journal) {
          if (event.sequence > cursor.afterSequence) {
            frames.push({ type: "event", event });
          }
        }
      }
      send({ type: "hello", protocolVersion: 1, epoch: hub.epoch });
      for (const frame of frames) {
        if (socket.readyState !== 1) {
          return;
        }
        send(frame);
      }
    });
    return socket;
  };
}

/** 마이크로태스크를 조건이 맞을 때까지 돌린다(시간 대기 없음). */
export async function until(condition: () => boolean, label = "condition", rounds = 10_000) {
  for (let round = 0; round < rounds; round += 1) {
    if (condition()) {
      return;
    }
    await Promise.resolve();
  }
  throw new Error(`timed out waiting for ${label}`);
}
