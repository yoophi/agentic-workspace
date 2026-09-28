"""비밀 파일의 창 토큰으로(그 창 Origin을 실어) handshake 한 번. 상태 코드만 출력한다(토큰은 출력하지 않음)."""
import json, sys, urllib.request, urllib.error, urllib.parse
s = json.load(open(sys.argv[1]))
host = urllib.parse.urlparse(s["baseUrl"]).hostname
if host not in ("127.0.0.1", "localhost", "::1"):
    print(json.dumps({"result": "refused-non-loopback"})); sys.exit(2)
req = urllib.request.Request(s["baseUrl"] + "/v1/system/handshake", method="POST",
    data=json.dumps({"supportedProtocolVersions": [1], "client": {"name": "token-check", "version": "044"}}).encode(),
    headers={"authorization": "Bearer " + s["token"], "content-type": "application/json", "origin": s["origin"]})
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
try:
    with opener.open(req, timeout=10) as r:
        print(json.dumps({"status": r.status}))
except urllib.error.HTTPError as e:
    body = e.read().decode(errors="replace")
    code = None
    try: code = json.loads(body).get("code") or json.loads(body).get("error", {}).get("code")
    except Exception: pass
    print(json.dumps({"status": e.code, "code": code}))
