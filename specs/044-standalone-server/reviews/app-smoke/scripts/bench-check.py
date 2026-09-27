"""소유자 bench.list에서 run이 있는지(identify 증명 확인 뒤). 토큰은 출력하지 않는다."""
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("oc", "/Users/yoophi/project/worktrees/044-standalone-server/specs/044-standalone-server/reviews/app-smoke/owner-check.py")
oc = importlib.util.module_from_spec(spec); spec.loader.exec_module(oc)
d = oc.load_descriptor(sys.argv[1]); run = sys.argv[2]
ok, _ = oc.verify_identity(d)
if not ok:
    print(json.dumps({"result": "identity-failed"})); sys.exit(2)
benches = oc.call(d["baseUrl"], d["ownerToken"], "bench.list", {}, False)
bench, found = oc.find_run(benches, run)
status = oc.call(d["baseUrl"], d["ownerToken"], "server.status", {}, False)
print(json.dumps({"result": "ok", "runListed": found is not None, "benchCount": len(benches), "leases": status.get("leases")}))
