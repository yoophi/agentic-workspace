// 043 T041: 시험 host(044 T017부터 `crates/workbench-host/examples/http_test_host.rs`)를 한 번 빌드한다.
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

export const REPO_ROOT = resolve(__dirname, "../../../../..");

export default function setup() {
  execFileSync("cargo", ["build", "-p", "workbench-host", "--example", "http_test_host", "--features", "test-hooks"], {
    cwd: REPO_ROOT,
    stdio: "inherit",
    env: { ...process.env, CARGO_INCREMENTAL: "0" },
  });
}
