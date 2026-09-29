# 047 Rust client/CLI 검증 안내

최신 종료 기준은 047 자체 구현·계약/actual server 검증·순차 OCR delegate → Codex adversarial --wait 리뷰 및 수정·PR/CI·squash merge·main checkout/pull·완료 인계 문서 후 중지다. 045/046·TUI/MCP·배포 등 후속 구현은 시작하지 않으며 미완료/gate 유지로 인계한다.

```sh
cargo build -p aw-cli
target/debug/aw operations
cargo test -p workbench-client -p aw-cli
cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings
scripts/test-workbench-client-wire.sh
```

기존 서버 descriptor는 `--descriptor`로 지정한다. mutation은 별도0700 caller retry state-dir를 요구한다. commands/input/retry/JSONL/exit/권한 및 이연 production gate는 [사용 안내](../../docs/workbench-rust-client-cli.md)를 따른다. ownerToken/ticket을 argv나 logs에 넣지 않는다.

actual wire script는 exact merged04420fcd5f source archive·별도 locked build의 binary SHA/provenance를 남기며 private temp data/control root의 서버1회 명시 실행과 실제 aw subprocess를 시험한다. 기본 unit 실행에서는 actual_server1개가 ignore이며 script의 명시적 --ignored 실행 결과를 별도로 기록한다. `--build-only`는 build/provenance만 수행하며 actual wire PASS로 계산하지 않는다.

확인 범위: readonly locator/identity/same-socket credential proof, malformed/oversize/redirect/proxy credential0, full fault/Unknown·explicit retry/CAS/crash reopen, applied cursor/old owner-generation/queue/recovery와 SIGINT/partial/broken stdout, shared OFD flags restoration 및 macOS /dev/null. controlled T032/033은 실제 recover request 수신 barrier 뒤 reply-first/events-first를 강제한다. actual event acceptance는 contracts/client.md D-C2/D-C3와 contracts/cli.md D-C4의 bootstrap s/r→runtimeReconciled s+1/r+1→notificationRecovery s+2/r+1 exact identity/ACK, HTTP reply와 final snapshot을 따른다. arbitrary sleep/느슨한 >= 비교나 같은 revision dedup으로 누락을 숨기지 않는다.

actual private fixture는 Main1/currentRunId null/active generation null 및 tasks/generations/reports/commands/coordinatorNotifications/dispatch0을 확인한 뒤 test-only empty recover를1회 실행한다. production recover 및 RunCancel gate를 해제하지 않는다. sandbox probe는 private sentinel 성공/home 파일 권한 거절/fork 거절을 같은 profile에서 bounded ownership으로 대조한다. 실제 process ownership/EOF/reap/child0 및 source/runtime 상태 근거를 기록하되045 production containment나 설치본/signed package readiness로 확대하지 않는다.

base20fcd5f와 미병합0456e4bf30/0469b1b2e2 및 기존 user files/worktree는 보존한다. macOS15.6.1 arm64 proof만 있으며14+/signed bundle/desktop·CLI·TUI matrix는 미검증이다. Linux/Windows는 제외한다. cancel-rejected exit6은 미충족 RunCancel prerequisite 때문에 production 미검증/이연으로 리뷰받는다.

최종 실제 명령/개수/exit와 실패·수정은 [validation.md](validation.md), 순차 리뷰/PR·merge는 [review-ledger.md](review-ledger.md), 작업 종료·후속 재개는 `docs/047-completion-handoff.md`를 따른다. 현재 최종 리뷰/PR/merge/인계 완료 전이다.
