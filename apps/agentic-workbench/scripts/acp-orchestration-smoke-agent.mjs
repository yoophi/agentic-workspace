#!/usr/bin/env node

import readline from "node:readline";

let sessionId = "orchestration-smoke-session";
let mcpRequestId = 1;
let waitingRole = null;

const input = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function send(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function respond(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

async function callTool(name, args) {
  const url = process.env.AW_MCP_URL;
  const token = process.env.AW_MCP_TOKEN;
  if (!url || !token) {
    throw new Error("AW MCP launch environment is unavailable");
  }
  const response = await fetch(url, {
    method: "POST",
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: mcpRequestId++,
      method: "tools/call",
      params: { name, arguments: args },
    }),
  });
  const payload = await response.json();
  const result = payload.result;
  if (!response.ok || result?.isError) {
    throw new Error(
      result?.structuredContent?.message ??
        `MCP tool ${name} failed with ${response.status}`,
    );
  }
  return result?.structuredContent;
}

function roleFromPrompt(prompt) {
  for (const role of ["Researcher", "Reviewer", "Tester"]) {
    if (prompt.includes(`Role: ${role}`)) {
      return role;
    }
  }
  return "Worker";
}

async function runTask(prompt) {
  const role = waitingRole ?? roleFromPrompt(prompt);
  const resumedFromInput = waitingRole !== null;
  waitingRole = null;
  await callTool("aw_report_progress", {
    requestId: `${role.toLowerCase()}-progress-1`,
    progressPercent: 25,
    summary: `${role} fixture가 작업 범위를 확인했습니다.`,
    findings: [],
  });
  await callTool("aw_report_progress", {
    requestId: `${role.toLowerCase()}-progress-2`,
    progressPercent: 70,
    summary: `${role} fixture가 결정적 근거를 수집했습니다.`,
    findings: [],
  });

  if (role === "Reviewer" && prompt.includes("[input]")) {
    await callTool("aw_request_parent_input", {
      requestId: "reviewer-input-1",
      summary: "검토 정책을 선택해야 합니다.",
      question: "엄격한 read-only 정책을 적용할까요?",
      options: ["strict", "report-only"],
    });
    waitingRole = role;
    return "Reviewer is waiting for parent input.";
  }
  if (role === "Tester" && prompt.includes("[fail]")) {
    await callTool("aw_report_blocked", {
      requestId: "tester-blocked-1",
      summary: "결정적 실패 fixture가 실행되었습니다.",
      findings: [],
    });
    return "Tester reported a deterministic blocked result.";
  }

  await callTool("aw_report_result", {
    requestId: `${role.toLowerCase()}-result-1`,
    summary: resumedFromInput
      ? `${role}가 부모 입력을 받아 완료한 결정적 구조화 결과입니다.`
      : `${role}의 결정적 구조화 결과입니다.`,
    findings: [
      {
        title: `${role} fixture finding`,
        detail: "오케스트레이션 결과의 출처와 역할을 검증합니다.",
        evidence: ["specs/033-agent-orchestration/quickstart.md"],
        severity: role === "Reviewer" ? "warning" : "info",
      },
    ],
    artifactRefs: [],
    unresolved: role === "Reviewer" ? ["policy-choice"] : [],
    confidence: 0.9,
  });
  return `${role} reported a structured result.`;
}

async function handleRequest(message) {
  const { id, method, params } = message;
  if (method === "initialize") {
    respond(id, {
      protocolVersion: 1,
      agentInfo: {
        name: "orchestration-smoke-agent",
        title: "Orchestration Smoke Agent",
        version: "0.1.0",
      },
      agentCapabilities: {},
    });
    return;
  }
  if (method === "session/new") {
    sessionId = `orchestration-smoke-${Date.now()}`;
    respond(id, { sessionId });
    return;
  }
  if (method === "session/set_config_option" || method === "session/set_mode") {
    respond(id, {});
    return;
  }
  if (method === "session/prompt") {
    const prompt = JSON.stringify(params?.prompt ?? params ?? "");
    const text = await runTask(prompt);
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: {
        sessionId: params?.sessionId ?? sessionId,
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text },
      },
    });
    respond(id, { stopReason: "end_turn" });
    return;
  }
  send({
    jsonrpc: "2.0",
    id,
    error: { code: -32601, message: `Unsupported method: ${method}` },
  });
}

input.on("line", (line) => {
  try {
    const message = JSON.parse(line);
    if (message.method && message.id !== undefined) {
      void handleRequest(message).catch((error) => {
        send({
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: -32603,
            message: error instanceof Error ? error.message : String(error),
          },
        });
      });
    }
  } catch (error) {
    console.error(`invalid JSON-RPC message: ${String(error)}`);
  }
});
