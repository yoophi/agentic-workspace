#!/usr/bin/env python3
"""042 시험용 최소 ACP agent(stdio, 줄 단위 JSON-RPC). prompt마다 권한을 요청하고 응답을 기다린다.

- 권한이 승인되면(`outcome: selected` + allow 선택지) 그 prompt를 `end_turn`으로 끝낸다.
- 권한 응답이 오류이거나 거절·취소면 `cancelled`로 끝낸다(실제 agent처럼 승인으로 보지 않는다).
- 취소 요청(`$/cancel_request`)이 오면 그 prompt를 `cancelled`로 끝낸다.
- 권한 요청 id는 prompt id에 대응한다 — 이미 끝난 prompt의 늦은 권한 응답은 무시한다.

`--log <path>`면 자기가 한 일을 **응답을 보내기 전에** 한 줄씩 기록·flush한다: `prompt:<id>`,
`prompt-text:<id>:<본문 JSON 문자열>`(043: 어떤 prompt가 몇 번 왔는지 대조),
`end_turn:<id>`, `cancelled:<id>:<cancel-request|permission-error|permission-refused>`. 시험이 terminal 결과가 어느
경로에서 왔는지 대조한다. 그 밖의 파일·네트워크는 건드리지 않는다.

044(작업 관문 시험):
- `--end-turn-gate <path>`: 권한이 승인된 prompt를 그 파일이 생길 때까지 끝내지 않는다(turn이 진행 중인 구간을 시험이
  정한다). 이 동안 다음 입력은 읽지 않는다 — 엔진이 대기열 prompt를 보내는 것은 현재 turn이 끝난 뒤다.
- `--after-gate-chunk`: `--end-turn-gate`의 문이 열린 뒤 그 파일 내용을 `after-gate:<내용>` agent 메시지로 보내고 turn을
  끝낸다(앱 종료 뒤 문을 연 쪽이 그 고유 출력을 live로 받는지 — T045 진행 중 turn 출력 지속).
- `--rpc-error-text <text>`: 본문에 text가 든 prompt에는 JSON-RPC 오류로 답한다(`rpc-error:<id>` 기록).
- `--respond-gate <path> --respond-gate-text <text>`: 본문에 text가 든 prompt는 `end_turn:<id>`를 기록한 **뒤** 그 파일이
  생길 때까지 응답을 보내지 않는다(기록은 보였지만 엔진은 아직 응답을 받지 못한 구간을 시험이 정한다)."""
import json
import os
import sys
import time

LOG = sys.argv[sys.argv.index("--log") + 1] if "--log" in sys.argv else None
# 043 앱 스모크: `--echo`면 받은 prompt 본문을 `echo:<본문>` agent 메시지로 먼저 돌려보낸다(앱이 그 출력을 받는지 확인).
ECHO = "--echo" in sys.argv
END_TURN_GATE = sys.argv[sys.argv.index("--end-turn-gate") + 1] if "--end-turn-gate" in sys.argv else None
AFTER_GATE_CHUNK = "--after-gate-chunk" in sys.argv
RPC_ERROR_TEXT = sys.argv[sys.argv.index("--rpc-error-text") + 1] if "--rpc-error-text" in sys.argv else None
RESPOND_GATE = sys.argv[sys.argv.index("--respond-gate") + 1] if "--respond-gate" in sys.argv else None
RESPOND_GATE_TEXT = sys.argv[sys.argv.index("--respond-gate-text") + 1] if "--respond-gate-text" in sys.argv else None
prompt_texts = {}
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


# 문 파일 대기의 상한(시험이 문을 만들기 전에 실패하면 agent가 영원히 남지 않게). 부모가 사라져도(launchd로 재부모화) 끝낸다.
GATE_WAIT_LIMIT_SECONDS = 120
PARENT = os.getppid()


def wait_for_gate(path):
    deadline = time.monotonic() + GATE_WAIT_LIMIT_SECONDS
    while not os.path.exists(path):
        if time.monotonic() > deadline or os.getppid() != PARENT:
            log(f"gate-abandoned:{os.path.basename(path)}")
            sys.exit(3)
        time.sleep(0.02)


def finish(prompt_id, stop_reason, reason=None):
    global pending_prompt
    log(f"{stop_reason}:{prompt_id}" if reason is None else f"cancelled:{prompt_id}:{reason}")
    if RESPOND_GATE and RESPOND_GATE_TEXT and RESPOND_GATE_TEXT in prompt_texts.get(prompt_id, ""):
        wait_for_gate(RESPOND_GATE)
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
        prompt_texts[pending_prompt] = text
        if RPC_ERROR_TEXT and RPC_ERROR_TEXT in text:
            log(f"rpc-error:{pending_prompt}")
            send({"jsonrpc": "2.0", "id": pending_prompt, "error": {"code": -32000, "message": "fake rpc error"}})
            pending_prompt = None
            continue
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
            if END_TURN_GATE:
                wait_for_gate(END_TURN_GATE)
                if AFTER_GATE_CHUNK:
                    with open(END_TURN_GATE) as gate:
                        token = gate.read().strip()
                    log(f"after-gate:{prompt_id}:{token}")
                    send({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": {
                            "sessionId": "fake-session",
                            "update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": f"after-gate:{token}"}},
                        },
                    })
            finish(prompt_id, "end_turn")
        else:
            finish(prompt_id, "cancelled", "permission-refused")
    elif "id" in message:
        respond(message["id"], {})
