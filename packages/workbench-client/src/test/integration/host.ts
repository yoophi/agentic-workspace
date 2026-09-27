// 043 T041: 시험 host 프로세스. 실제 042 router·런타임·hub(작은 보관 한도)를 루프백에 띄우고, 준비되면 stdout 첫 줄에 JSON을
// 낸다. stdin을 닫으면 우아하게 끝난다.
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { resolve } from "node:path";

import { createConnection, type Connection } from "../../connection";

const BINARY = resolve(__dirname, "../../../../../target/debug/examples/http_test_host");

export interface HostInfo {
  baseUrl: string;
  epoch: string;
  dataDir: string;
  workDir: string;
  /** `owner`: 소유자 principal(044 — `server.*`·`lease.*`). */
  tokens: { windowA: string; windowB: string; owner: string };
}

export interface Host extends HostInfo {
  process: ChildProcessWithoutNullStreams;
  stop(): Promise<void>;
}

export function startHost(env: Record<string, string> = {}): Promise<Host> {
  const child = spawn(BINARY, [], { env: { ...process.env, ...env } });
  return new Promise((resolveHost, reject) => {
    let buffer = "";
    const onData = (chunk: Buffer) => {
      buffer += chunk.toString("utf8");
      const newline = buffer.indexOf("\n");
      if (newline < 0) {
        return;
      }
      child.stdout.off("data", onData);
      const info = JSON.parse(buffer.slice(0, newline)) as HostInfo;
      const exited = new Promise<void>((done) => child.once("exit", () => done()));
      resolveHost({
        ...info,
        process: child,
        stop: async () => {
          if (child.exitCode === null) {
            child.stdin.end();
          }
          await exited;
        },
      });
    };
    child.stdout.on("data", onData);
    child.once("error", reject);
    child.once("exit", (code) => reject(new Error(`host exited early with ${code}`)));
  });
}

/** 창 A 토큰으로 host에 붙은 연결. `target`을 바꾸면 다음 연결 정보 요청부터 새 host를 가리킨다(서버 재기동). */
export async function connectTo(target: { current: HostInfo }, token: keyof HostInfo["tokens"] = "windowA"): Promise<Connection> {
  const connection = createConnection({
    fetchConnection: async () => ({
      baseUrl: target.current.baseUrl,
      token: target.current.tokens[token],
      expiresAt: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
      incarnation: "inc-1",
    }),
  });
  await connection.start();
  return connection;
}
