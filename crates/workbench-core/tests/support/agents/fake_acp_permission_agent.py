#!/usr/bin/env python3
"""042 시험용 최소 ACP agent(stdio, 줄 단위 JSON-RPC). prompt마다 권한을 요청하고 응답을 기다린다.
취소(`$/cancel_request`)가 오면 그 prompt를 `cancelled`로 끝낸다. 파일·네트워크를 건드리지 않는다."""
import json
import sys

pending_prompt = None
permission_seq = 0


def send(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def respond(request_id, result):
    send({"jsonrpc": "2.0", "id": request_id, "result": result})


def ask_permission():
    global permission_seq
    permission_seq += 1
    send({
        "jsonrpc": "2.0",
        "id": f"perm-{permission_seq}",
        "method": "session/request_permission",
        "params": {
            "sessionId": "fake-session",
            "toolCall": {"toolCallId": f"tool-{permission_seq}", "title": "write a file"},
            "options": [
                {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
                {"optionId": "reject", "name": "Reject", "kind": "reject_once"},
            ],
        },
    })


for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    message = json.loads(line)
    method = message.get("method")
    if method == "initialize":
        respond(message["id"], {
            "protocolVersion": 1,
            "agentCapabilities": {},
            "authMethods": [],
            "agentInfo": {"name": "fake-acp", "version": "0"},
        })
    elif method == "session/new":
        respond(message["id"], {"sessionId": "fake-session"})
    elif method == "session/prompt":
        pending_prompt = message["id"]
        ask_permission()
    elif method == "$/cancel_request":
        if pending_prompt is not None and message.get("params", {}).get("requestId") == pending_prompt:
            respond(pending_prompt, {"stopReason": "cancelled"})
            pending_prompt = None
    elif method is None and str(message.get("id", "")).startswith("perm-"):
        if pending_prompt is not None:
            respond(pending_prompt, {"stopReason": "end_turn"})
            pending_prompt = None
    elif "id" in message:
        respond(message["id"], {})
