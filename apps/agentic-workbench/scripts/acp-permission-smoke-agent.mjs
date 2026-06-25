#!/usr/bin/env node

import readline from "node:readline";

let nextRequestId = 1;
const pending = new Map();
let sessionId = "smoke-session";

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function send(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function respond(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function request(method, params) {
  const id = nextRequestId++;
  send({ jsonrpc: "2.0", id, method, params });
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
  });
}

function resolvePending(message) {
  if (message.id === undefined || message.method) {
    return false;
  }
  const waiter = pending.get(message.id);
  if (!waiter) {
    return false;
  }
  pending.delete(message.id);
  if (message.error) {
    waiter.reject(new Error(message.error.message ?? "JSON-RPC error"));
  } else {
    waiter.resolve(message.result);
  }
  return true;
}

async function handleRequest(message) {
  const { id, method, params } = message;

  if (method === "initialize") {
    respond(id, {
      protocolVersion: 1,
      agentInfo: {
        name: "permission-smoke-agent",
        title: "Permission Smoke Agent",
        version: "0.1.0",
      },
      agentCapabilities: {},
    });
    return;
  }

  if (method === "session/new") {
    sessionId = `smoke-session-${Date.now()}`;
    respond(id, { sessionId });
    return;
  }

  if (method === "session/set_config_option" || method === "session/set_mode") {
    respond(id, {});
    return;
  }

  if (method === "session/prompt") {
    await handlePrompt(id, params);
    return;
  }

  send({
    jsonrpc: "2.0",
    id,
    error: {
      code: -32601,
      message: `Unsupported method: ${method}`,
    },
  });
}

async function handlePrompt(id, params) {
  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: {
      sessionId: params?.sessionId ?? sessionId,
      sessionUpdate: "agent_message_chunk",
      content: { type: "text", text: "Requesting smoke-test permission." },
    },
  });

  const permissionResult = await request("session/request_permission", {
    sessionId: params?.sessionId ?? sessionId,
    toolCall: {
      title: "Smoke permission request",
      rawInput: {
        action: "verify-permission-round-trip",
        workspace: process.cwd(),
      },
    },
    options: [
      {
        optionId: "allow-once",
        name: "Allow once",
        kind: "allow_once",
      },
      {
        optionId: "reject",
        name: "Reject",
        kind: "reject_once",
      },
    ],
  });

  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: {
      sessionId: params?.sessionId ?? sessionId,
      sessionUpdate: "agent_message_chunk",
      content: {
        type: "text",
        text: `Permission response: ${JSON.stringify(permissionResult)}`,
      },
    },
  });

  respond(id, { stopReason: "end_turn" });
  process.exitCode = 0;
  setTimeout(() => process.exit(0), 50);
}

rl.on("line", (line) => {
  let message;
  try {
    message = JSON.parse(line);
  } catch (error) {
    console.error(`invalid JSON-RPC message: ${error}`);
    return;
  }

  if (resolvePending(message)) {
    return;
  }
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
});
