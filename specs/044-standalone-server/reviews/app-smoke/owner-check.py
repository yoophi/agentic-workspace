#!/usr/bin/env python3
"""044 T035: 데스크톱 없이 소유자 자격 증명으로 run을 조회·관찰·취소하는 스모크 클라이언트(표준 라이브러리만).

순서가 계약이다(research R5·R6, contracts/server-lifecycle.md §3):
1. `<data-dir>/workbench/server/server.json`을 읽는다.
2. 인증 없는 `/v1/system/identify`로 신원 증명을 받아 안내 파일의 `ownerToken`으로 검증한다
   (`HMAC-SHA256(ownerToken, nonce + "\\n" + instanceId)`, hex). **틀리면 자격 증명을 보내지 않고 끝낸다(종료 코드 2).**
3. 소유자 토큰으로 handshake(세대) → `bench.list`(run 찾기) → `run.replay`(지난 출력) →
   이벤트 표 + WebSocket 구독(마지막 순번 뒤) → 소유자 `run.sendPrompt` → 그 prompt의 출력이 구독으로 오는지 →
   `run.cancel` → run이 `bench.list`에서 빠지는지.

결과 JSON을 표준 출력에 쓴다(토큰·표 문자열 없음). 성공이면 종료 코드 0, 실패 1, 신원 증명 실패 2, 사용법 64.
"""

import argparse
import base64
import hashlib
import hmac
import json
import os
import socket
import sys
import time
import urllib.error
import urllib.request
import uuid

PROTOCOL_VERSION = 1


class CheckError(Exception):
    pass


def load_descriptor(data_dir):
    path = os.path.join(data_dir, "workbench", "server", "server.json")
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def http_json(base_url, method, path, body=None, bearer=None, timeout=10):
    if not (base_url.startswith("http://127.0.0.1:") or base_url.startswith("http://localhost:")):
        raise CheckError(f"refusing a non-loopback server address {base_url}")
    data = None if body is None else json.dumps(body).encode("utf-8")
    request = urllib.request.Request(base_url + path, data=data, method=method)
    request.add_header("accept", "application/json")
    if data is not None:
        request.add_header("content-type", "application/json")
    if bearer is not None:
        request.add_header("authorization", f"Bearer {bearer}")
    # 소유자 요청에는 Origin을 싣지 않는다(서버는 Origin이 있는 소유자 자격 증명을 거절한다).
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(request, timeout=timeout) as response:
            raw = response.read()
            status = response.status
    except urllib.error.HTTPError as error:
        raw = error.read()
        status = error.code
    try:
        payload = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        payload = None
    return status, payload


def call(base_url, owner, operation, payload, command):
    envelope = {
        "protocolVersion": PROTOCOL_VERSION,
        "operation": operation,
        "requestId": "req_" + uuid.uuid4().hex,
        "input": payload,
    }
    if command:
        envelope["idempotencyKey"] = "idem_" + uuid.uuid4().hex
    status, body = http_json(base_url, "POST", "/v1/calls", envelope, owner)
    if status == 200 and isinstance(body, dict) and body.get("kind") == "complete":
        return body.get("output")
    code = body.get("code") if isinstance(body, dict) else None
    message = body.get("message") if isinstance(body, dict) else None
    raise CheckError(f"{operation} failed: HTTP {status} {code} {message}")


def verify_identity(descriptor):
    nonce = uuid.uuid4().hex
    status, body = http_json(descriptor["baseUrl"], "POST", "/v1/system/identify", {"nonce": nonce})
    if status != 200 or not isinstance(body, dict):
        return False, f"identify answered {status}"
    instance = body.get("instanceId", "")
    expected = hmac.new(
        descriptor["ownerToken"].encode("utf-8"),
        f"{nonce}\n{instance}".encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()
    if instance != descriptor["instanceId"] or not hmac.compare_digest(expected, body.get("proof", "")):
        return False, "the endpoint is not the descriptor's server instance"
    return True, instance


class WebSocket:
    """최소 WebSocket 클라이언트(RFC 6455): 텍스트·ping·close만 다룬다. 서버 프레임은 가리지 않고, 클라이언트 프레임은 가린다."""

    def __init__(self, base_url, path, timeout):
        authority = base_url[len("http://"):]
        host, port = authority.rsplit(":", 1)
        self.sock = socket.create_connection((host, int(port)), timeout=timeout)
        self.sock.settimeout(timeout)
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = (
            f"GET {path} HTTP/1.1\r\nHost: {authority}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(request.encode("ascii"))
        head = b""
        while b"\r\n\r\n" not in head:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise CheckError("websocket handshake closed")
            head += chunk
        header, self.buffer = head.split(b"\r\n\r\n", 1)
        status_line = header.split(b"\r\n", 1)[0].decode("latin-1")
        if " 101 " not in status_line:
            raise CheckError(f"websocket upgrade refused: {status_line}")
        accept = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()
        ).decode("ascii")
        if accept.encode("ascii") not in header:
            raise CheckError("websocket accept key mismatch")

    def _read(self, count):
        while len(self.buffer) < count:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise CheckError("websocket closed")
            self.buffer += chunk
        data, self.buffer = self.buffer[:count], self.buffer[count:]
        return data

    def _send(self, opcode, payload=b""):
        mask = os.urandom(4)
        header = bytes([0x80 | opcode])
        length = len(payload)
        if length < 126:
            header += bytes([0x80 | length])
        elif length < 65536:
            header += bytes([0x80 | 126]) + length.to_bytes(2, "big")
        else:
            header += bytes([0x80 | 127]) + length.to_bytes(8, "big")
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.sock.sendall(header + mask + masked)

    def next_text(self):
        message = b""
        while True:
            first, second = self._read(2)
            opcode = first & 0x0F
            fin = bool(first & 0x80)
            length = second & 0x7F
            if length == 126:
                length = int.from_bytes(self._read(2), "big")
            elif length == 127:
                length = int.from_bytes(self._read(8), "big")
            if second & 0x80:
                mask = self._read(4)
                payload = bytes(byte ^ mask[i % 4] for i, byte in enumerate(self._read(length)))
            else:
                payload = self._read(length)
            if opcode == 0x9:
                self._send(0xA, payload)
                continue
            if opcode == 0xA:
                continue
            if opcode == 0x8:
                raise CheckError("websocket closed by the server")
            if opcode in (0x1, 0x0):
                message += payload
                if fin:
                    return json.loads(message.decode("utf-8"))

    def close(self):
        try:
            self._send(0x8)
        except OSError:
            pass
        self.sock.close()


def is_completion(body):
    """run 스트림의 prompt 완료 이벤트(lifecycle promptCompleted)."""
    return body.get("type") == "lifecycle" and body.get("status") == "promptCompleted"


def find_run(benches, run_id):
    for bench in benches:
        for run in bench.get("runs", []):
            if run_id is None or run.get("runId") == run_id:
                return bench["benchId"], run["runId"]
    return None, None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--data-dir", required=True)
    parser.add_argument("--run-id")
    parser.add_argument("--prompt")
    parser.add_argument("--timeout", type=float, default=20.0)
    # T045(SC-001, Codex 문서 리뷰): 새 prompt를 보내지 않고 **진행 중인 기존 turn**의 완료를 live로 본다. 구독을 연 뒤
    # `--release-file`을 만들어 agent의 turn 문을 푼다(구독 전에 완료되지 않게).
    parser.add_argument("--observe-turn", action="store_true")
    parser.add_argument("--release-file")
    args = parser.parse_args()
    report = {"result": "error", "steps": {}}
    try:
        descriptor = load_descriptor(args.data_dir)
        report["instanceId"] = descriptor.get("instanceId")
        verified, detail = verify_identity(descriptor)
        report["steps"]["identify"] = "ok" if verified else "failed"
        if not verified:
            report["result"] = "identity-failed"
            report["error"] = detail
            print(json.dumps(report))
            return 2
        base = descriptor["baseUrl"]
        owner = descriptor["ownerToken"]

        status, handshake = http_json(
            base,
            "POST",
            "/v1/system/handshake",
            {"supportedProtocolVersions": [PROTOCOL_VERSION], "client": {"name": "owner-check", "version": "044"}},
            owner,
        )
        if status != 200 or handshake.get("instanceId") != descriptor["instanceId"]:
            raise CheckError(f"handshake failed: HTTP {status}")
        epoch = handshake["serverEpoch"]
        report["steps"]["handshake"] = "ok"

        benches = call(base, owner, "bench.list", {}, False)
        bench_id, run_id = find_run(benches, args.run_id)
        if run_id is None:
            raise CheckError(f"run {args.run_id or '(any)'} is not listed: {benches}")
        report["benchId"] = bench_id
        report["runId"] = run_id
        report["steps"]["benchList"] = "ok"

        replay = call(base, owner, "run.replay", {"benchId": bench_id, "runId": run_id, "afterSequence": 0}, False)
        if args.observe_turn:
            bodies = [(item.get("event") or item.get("body") or {}) for item in replay.get("events", [])]
            report["steps"]["replayHasCompletion"] = any(is_completion(body) for body in bodies)
            if report["steps"]["replayHasCompletion"]:
                raise CheckError("the start turn already completed before the observation (not an in-flight turn)")
        sequences = [item["sequence"] for item in replay.get("events", [])]
        last = replay.get("lastSequence", 0)
        report["steps"]["replayEvents"] = len(sequences)
        report["steps"]["replayLastSequence"] = last
        report["steps"]["replayContiguous"] = sequences == list(range(1, len(sequences) + 1))
        if not sequences:
            raise CheckError("the replay has no events")

        status, ticket = http_json(
            base,
            "POST",
            "/v1/event-tickets",
            {"cursors": [{"streamId": f"run:{run_id}", "epoch": epoch, "afterSequence": last}]},
            owner,
        )
        if status != 200:
            raise CheckError(f"event ticket failed: HTTP {status} {ticket}")
        socket_client = WebSocket(base, "/v1/events?ticket=" + ticket["ticket"], args.timeout)
        try:
            hello = socket_client.next_text()
            if hello.get("type") != "hello":
                raise CheckError(f"expected hello, got {hello}")
            live = []
            echoed = False
            deadline = time.monotonic() + args.timeout
            if args.observe_turn:
                # 고유 표지를 문 파일에 쓴다. agent(`--after-gate-chunk`)가 문이 열린 뒤 `after-gate:<표지>`를 출력한다 —
                # 앱 종료 뒤에 만든 출력이라는 증거다(종료 전 replay에는 있을 수 없다).
                marker = "owner-release-" + uuid.uuid4().hex[:8]
                if args.release_file:
                    # 원자적 쓰기: agent는 파일이 생기는 순간 읽으므로 빈 내용을 보지 않게 임시 파일에 쓴 뒤 이름을 바꾼다.
                    staging = args.release_file + ".tmp"
                    with open(staging, "w") as gate:
                        gate.write(marker)
                    os.replace(staging, args.release_file)
                report["steps"]["promptSent"] = False
                chunk = False
                while time.monotonic() < deadline and not echoed:
                    frame = socket_client.next_text()
                    if frame.get("type") != "event":
                        continue
                    event = frame["event"]
                    live.append(event["sequence"])
                    body = event.get("body") or {}
                    if body.get("type") == "agentMessage" and body.get("text") == "after-gate:" + marker:
                        chunk = True
                    if is_completion(body):
                        echoed = True
                report["steps"]["liveCompletion"] = echoed
                report["steps"]["liveOutputAfterRelease"] = chunk
                # 완료만이 아니라 앱 종료 뒤 새 출력까지 받아야 한다.
                echoed = echoed and chunk
            prompt = args.prompt or ("owner-check-" + uuid.uuid4().hex[:8])
            if not args.observe_turn:
                call(base, owner, "run.sendPrompt", {"benchId": bench_id, "runId": run_id, "prompt": prompt}, True)
            while not args.observe_turn and time.monotonic() < deadline and not echoed:
                frame = socket_client.next_text()
                if frame.get("type") != "event":
                    continue
                event = frame["event"]
                live.append(event["sequence"])
                body = event.get("body") or {}
                text = body.get("text")
                if body.get("type") == "agentMessage" and isinstance(text, str) and text.endswith(prompt):
                    echoed = True
        finally:
            socket_client.close()
        report["steps"]["liveSequences"] = live
        report["steps"]["liveAfterReplay"] = bool(live) and all(sequence > last for sequence in live)
        report["steps"]["liveEcho" if not args.observe_turn else "liveTurnCompleted"] = echoed
        if not echoed:
            raise CheckError("no live completion of the in-flight turn" if args.observe_turn else "no live output for the owner's prompt")

        call(base, owner, "run.cancel", {"benchId": bench_id, "runId": run_id}, True)
        deadline = time.monotonic() + args.timeout
        gone = False
        while time.monotonic() < deadline:
            _, still = find_run(call(base, owner, "bench.list", {}, False), run_id)
            if still is None:
                gone = True
                break
            time.sleep(0.1)
        report["steps"]["cancelled"] = gone
        if not gone:
            raise CheckError("the cancelled run is still listed")
        report["result"] = "ok" if report["steps"]["liveAfterReplay"] else "failed"
    except (CheckError, OSError, KeyError, ValueError) as error:
        report["result"] = "error"
        report["error"] = str(error)
    print(json.dumps(report))
    return 0 if report["result"] == "ok" else 1


if __name__ == "__main__":
    sys.exit(main())
