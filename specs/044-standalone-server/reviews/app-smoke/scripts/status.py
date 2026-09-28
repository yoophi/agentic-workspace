"""소유자 server.status 한 번(identify 증명 확인 뒤). owner-check.py의 함수를 재사용한다. 토큰은 출력하지 않는다."""
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("oc", "/Users/yoophi/project/worktrees/044-standalone-server/specs/044-standalone-server/reviews/app-smoke/owner-check.py")
oc = importlib.util.module_from_spec(spec); spec.loader.exec_module(oc)
d = oc.load_descriptor(sys.argv[1])
ok, detail = oc.verify_identity(d)
if not ok:
    print(json.dumps({"result": "identity-failed"})); sys.exit(2)
s = oc.call(d["baseUrl"], d["ownerToken"], "server.status", {}, False)
print(json.dumps({"result": "ok", "state": s.get("state"), "leases": s.get("leases"), "busyRuns": s.get("activeWork", {}).get("busyRuns")}))
