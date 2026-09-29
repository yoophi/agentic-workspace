# 047 Rust client/CLI 완료 인계

이 문서는 047 Rust client/CLI 범위의 구현·검증·리뷰·PR/CI·squash merge·main 동기화 완료를 기록한다. 전체 AW 서버·thin desktop 전환 완료를 의미하지 않는다. 사용자의 2026-09-29 최신 지시에 따라047 merge/main 동기화 및 이 기록 완료 후 작업을 중지하며, 후속 구현은 시작하지 않는다.

## 완료 범위

- 독립 `workbench-client` 및 `aw` CLI: existing-instance descriptor·신원·HTTP/WS 연결, allowlist 조회/비실행 mutation, 일반 호출과 명시 명령 parity.
- 전송 전 private durable retry state, 동일 key/input/instance/epoch의 명시 재시도, cancellation-safe Unknown 및 late completion CAS 격리, terminal 완료 캐시.
- 반환 eventStreamId 구독과 ACK cursor, replay/live interleave, notification live-first snapshot/all-reset, epoch/exhaustion stale completion, aggregate queue quota 및 bounded reconnect/callback/socket ownership.
- finite JSON/JSONL, stdin·SIGINT·timeout·panic redaction, 공유 stdout flags lease/복구와 `/dev/null`, 실제 subprocess cleanup.
- exact merged044 서버 독립 binary/private root와 실제 aw wire 검증. test-only empty recover authority 및 seatbelt fixture는 production activation과 구분한다.

## 커밋과 PR

구현 PR: [#209](https://github.com/yoophi/agentic-workspace/pull/209), 최종 branch HEAD9947fbc38caa7fd3e9c72d778ea490239557dcc6. 검증·승인 실행 source는 f8df5dc4929c637c3799ac488849736fce76e890이며9947fbc는 결과 기록 문서만 변경했다. PR #209는 squash merge 완료이며 실제 SHA는 `c3973ee5534850a66673867ce629b91ab3b67f9f`다.
`git checkout main` 및 `git pull --ff-only origin main`(command-only HTTPS rewrite/credential helper/curloptResolve)를 실제 exit0으로 수행했다. local HEAD와 origin/main은 모두 `c3973ee5534850a66673867ce629b91ab3b67f9f`다. merged source의 crates/apps/scripts/Cargo bytes가 승인된 f8df5dc와 동일함을 git diff exit0으로 확인했다.
최종 구현 리뷰는 OCR delegate→Codex adversarial `--wait` 순서로 수행했고 원 needs-attention 결과와 유효 수정 이력은 review ledger에 보존한다. OCR73/73(skipped0)→Codex6 thread01a0ed67-c568-7ad2-b033-ebda44f85646 /exec91224 actualexit0/approve. 원 needs-attention은 round1–5에 보존하고 I-O1/O2 및 I-C1–C6를 수정·재검증했다.

## 검증 근거

실행 host는 macOS15.6.1(24G90)이다. Linux/Windows는 이번 범위에서 제외했다. 상세 실제 명령·exit·실패/수정·fixture 제한은 [validation](../specs/047-rust-client-cli/validation.md), [리뷰 기록](../specs/047-rust-client-cli/review-ledger.md), [체크리스트](../specs/047-rust-client-cli/tasks.md)에 기록했다.

고정 f8df5dc root8 all exit0: workspace120suites/1234passed/failed0/ignored8, strictClippy/fmt, pnpm check-types/test/build, client integration3files7tests 및 AW integration3files14tests. [PR CI Quality run36577684368/job109437417847](https://github.com/yoophi/agentic-workspace/actions/runs/36577684368/job/109437417847)은 COMPLETED/SUCCESS, completedAt2026-09-29T14:01:40Z다. worker connector로 모든 step success와 원격HEAD9947fbc를 확인했다.
client/CLI215passed (client153/CLI62)/failed0/ignored actual1, 별도 실제 서버1passed. exact baseline20fcd5fdcf633ae06792d51a9b963e3857909440 serverSHA3fcb07ef711a77ffdc58f14c60f144455a7d4cfe3ec9cb5a83011fa67b425e9d. private-root bootstrap s1/r0→runtimeReconciled s2/r1·notificationRecovery s3/r1, ACK3/final snapshot1, project CRUD same-key effect1 및 bounded reap/child0을 확인했다.

## 미완료 선행 조건과 유지한 gate

045의 macOS production containment/legacy migration(T010/T016)과046 lifecycle·단일 writer·freeze/backup/restore proof는 미완료다. 기존 branches/worktrees를 보존했고 이 작업에 가져오거나 구현하지 않았다. private sentinel/home/fork fixture 대조는 실제 production signing/launch/migration proof가 아니다.

RunCancel·agent/run/terminal/Git/helper·ensure/stop/recover 활성화는 기존 prerequisiteUnavailable 정책을 유지한다. T016의 cancel-rejected production exit6은 도달 불가능하여 명시 이연했으며 explicit/generic gate parity만 증명했다. server 자체가 operation을 제공한다는 사실을 activation readiness로 사용하지 않는다.

TUI/MCP, actual installed desktop 종료 뒤 run 관찰/취소 및 desktop/CLI/TUI concurrent matrix, macOS14+ 전체 검증, signed/notarized CALVER packaging·discovery/update, desktop business fallback 제거는 미완료다. 원 전체 전환 목표는 사용자 소유 untracked goal.md의 superseded 이력에 보존했다.

## 관측한 연결 장애와 실제 복구

로컬 gh의 API GraphQL/REST 및 curl IPv4에서 반복 TLS handshake/SSL connection timeout을 관측했다. GitHub Chrome 직접 이동도 navigation timeout이었다. read-only GitHub connector는 PR currentHEAD9947fbc/open/mergeable 및 CI success를 조회했으나 merge write는403 Resource not accessible by integration이었다. 이는 connector의 쓰기 capability 문제이며 사용자 gh ADMIN 권한 부족으로 단정하지 않는다. 원격 인증·remote·보안 설정은 변경하지 않았다.

HTTPS DNS 조회에서 얻은 다른 API endpoint를 TLS 인증서/hostname 검증을 유지한 요청별 curl resolve로 probe하여 HTTP200을 확인했다. 기존 gh 인증은 프로세스 내부 stdin config로만 사용하고 argv/log/document에 출력하지 않았다. 요청 직전에 PR head9947fbc/open과 CI 전체 success를 다시 검증했으며 expected sha를 지정한 REST squash 응답은 merged=true/실제SHAc3973ee였다. 이어 요청별 Git curloptResolve/HTTPS rewrite로 main pull exit0/local=origin main을 확인했다. IP는 임시 연결 증거이며 장기 고정 주소로 사용하지 않는다. 시스템 DNS/인증/remote/SSL 검증 설정 변경0이다. T043 인계와 T044 구현PR/CI·merge/main sync·최종 기록 조건은 충족됐고 후속 구현은 시작하지 않는다.

## 후속 작업과 재개 방법

새 사용자 지시가 있을 때만 후속 작업을 시작한다. main에서 AGENTS.md→openwiki/quickstart.md→goal.md→해당 specs를 읽고 git/user files 상태를 먼저 확인한다. 045 production containment와046 lifecycle/data gates의 실제 증거를 확보한 뒤 TUI/MCP·배포·fallback 전환 순서를 새 범위로 설계·리뷰한다. 기존 readiness를 미검증 boolean으로 활성화하거나 fixture 검증을 전체 rollout 완료로 축약하지 않는다.

047 재검증은 root package 8개 gate와 `scripts/test-workbench-client-wire.sh`를 사용한다. script는 exact20fcd5f archive 서버를 독립 locked build하며 actual_server ignored 시험을 명시 실행한다. `/private/tmp/aw-047-design`의 원 리뷰/실행 artifacts는 임시 자료이므로 재개 시 존재 여부를 확인한다. repo 문서의 명령·SHA·결과를 기본 기록으로 사용하고 production user root/기존 daemon을 fixture에 사용하지 않는다.

사용자 별도 문서 `docs/code-review-app-migration.md`, `docs/code-oss-chat-aw-binding-plan.md`와 untracked goal.md를 커밋하지 않았다. 기존 branches/worktrees/user files를 보존했다.
