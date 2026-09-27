#!/usr/bin/env python3
"""042 시험용 최소 ACP agent(stdio, 줄 단위 JSON-RPC). prompt마다 권한을 요청하고 응답을 기다린다.

- 권한이 승인되면(`outcome: selected` + allow 선택지) 그 prompt를 `end_turn`으로 끝낸다.
- 권한 응답이 오류이거나 거절·취소면 `cancelled`로 끝낸다(실제 agent처럼 승인으로 보지 않는다).
- 취소 요청(`$/cancel_request`)이 오면 그 prompt를 `cancelled`로 끝낸다.
- 권한 요청 id는 prompt id에 대응한다 — 이미 끝난 prompt의 늦은 권한 응답은 무시한다.

`--log <path>`면 자기가 한 일을 **응답을 보내기 전에** 한 줄씩 기록·flush한다: `prompt:<id>`,
`prompt-text:<id>:<본문 JSON 문자열>`(043: 어떤 prompt가 몇 번 왔는지 대조),
`end_turn:<id>`, `cancelled:<id>:<cancel-request|permission-error|permission-refused>`. 시험이 terminal 결과가 어느
경로에서 왔는지 대조한다. 그 밖의 파일·네트워크는 건드리지 않는다."""
import json
import sys

LOG = sys.argv[sys.argv.index("--log") + 1] if "--log" in sys.argv else None
# 043 앱 스모크: `--echo`면 받은 prompt 본문을 `echo:<본문>` agent 메시지로 먼저 돌려보낸다(앱이 그 출력을 받는지 확인).
ECHO = "--echo" in sys.argv
pending_prompt = None
prompt_for_permission = {}
permission_seq = 0


def log(entry):
    """응답보다 먼저 부른다. flush 뒤 닫으므로 응답을 받은 런타임이 곧바로 프로세스를 끝내도 기록이 남는다."""
    if LOG:
        with open(LOG, "a") as handle:
            handle.write(entry + "\n")
            handle.flush()


def send(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def respond(request_id, result):
    send({"jsonrpc": "2.0", "id": request_id, "result": result})


def finish(prompt_id, stop_reason, reason=None):
    global pending_prompt
    log(f"{stop_reason}:{prompt_id}" if reason is None else f"cancelled:{prompt_id}:{reason}")
    respond(prompt_id, {"stopReason": "end_turn" if stop_reason == "end_turn" else "cancelled"})
    if pending_prompt == prompt_id:
        pending_prompt = None


def ask_permission(prompt_id):
    global permission_seq
    permission_seq += 1
    permission_id = f"perm-{permission_seq}"
    prompt_for_permission[permission_id] = prompt_id
    send({
        "jsonrpc": "2.0",
        "id": permission_id,
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


def granted(message):
    outcome = (message.get("result") or {}).get("outcome") or {}
    return outcome.get("outcome") == "selected" and outcome.get("optionId") == "allow"


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
        log(f"prompt:{pending_prompt}")
        blocks = (message.get("params") or {}).get("prompt") or []
        text = " ".join(block.get("text", "") for block in blocks if isinstance(block, dict))
        log(f"prompt-text:{pending_prompt}:{json.dumps(text)}")
        if ECHO:
            send({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": "fake-session",
                    "update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": f"echo:{text}"}},
                },
            })
        ask_permission(pending_prompt)
    elif method == "$/cancel_request":
        if pending_prompt is not None and message.get("params", {}).get("requestId") == pending_prompt:
            finish(pending_prompt, "cancelled", "cancel-request")
    elif method is None and str(message.get("id", "")) in prompt_for_permission:
        prompt_id = prompt_for_permission.pop(str(message["id"]))
        if prompt_id != pending_prompt:
            continue  # 이미 끝난 prompt의 늦은 권한 응답
        if "error" in message:
            finish(prompt_id, "cancelled", "permission-error")
        elif granted(message):
            finish(prompt_id, "end_turn")
        else:
            finish(prompt_id, "cancelled", "permission-refused")
    elif "id" in message:
        respond(message["id"], {})
