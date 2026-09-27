# Quickstart: Workbench HTTP/WebSocket 어댑터 (042) 검증

## 1. 자동 검증 (한 번 실행, 원 명령 종료 코드 기록)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets --no-fail-fast
(cd apps/agentic-workbench/src-tauri && cargo check --release)
pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated
pnpm run check-types
pnpm run test
```

기대:

- 계약 suite·이벤트 suite가 **운영 router**(`workbench-server`)를 임의 루프백 포트로 띄워 모든 fixture를 in-memory와 같은 결과로 통과한다(차이 0).
- 보안 거절(contracts §1·§2·§4): 허용 안 된 Host/Origin, 접두사만 같은 Origin, `null` Origin, 토큰 없음·잘못됨·만료, 데스크톱 토큰을 다른 Origin·Origin 없이 사용, 재사용·만료·다른 Origin의 표, 1 MiB 초과, 폐기된 MCP 토큰 → 모두 거절, 뒤이은 상태 조회에서 변화 없음.
- WebSocket 경계 경합: 표로 구독하는 순간에 발행을 주입하는 시험 1,000회 이상에서 빠짐·중복 0.
- 경로 혼합 동시 변경: in-process와 HTTP로 같은 대상에 100회 이상 → 손실 0.
- 중단·재시작 증거(research R13): 보강한 영속 5개(`project.update`·`project.delete`·`savedPrompt.update`·`goal.update`·`goal.clear`)의 세 중단 지점 판정, 세대 범위·orchestration 변경의 재시작 뒤 같은 키 재시도 → 재적용 없음. 각 재시도는 HTTP로도 한 번 보낸다.
- 기록 수집 sink에 토큰·표 문자열 0건.
- MCP `origin_allowed`: `http://127.0.0.1.evil.example`·`http://localhost.evil.example`·`null` 거절, 허용 목록 통과.

## 2. 경계 확인

```bash
git diff --stat origin/main -- apps/agentic-workbench/src crates/acp-agent-core packages/agent-client   # 0
grep -rn "axum\|tower" crates/workbench-core/Cargo.toml   # dev-dependencies만
(cd apps/agentic-workbench/src-tauri && cargo build --release) && \
  strings target/release/agentic-workbench | grep -c "AW_HTTP_DIAGNOSTIC_FILE\|AW_HTTP_WEBVIEW_PROBE_FILE\|report_http_probe"   # 0 (debug 전용 경로가 release에 없다)
```

## 3. 앱 연결 스모크 (자동화 — 격리 데이터, 두 증거 구분)

설치된 AW가 같은 identifier로 실행 중일 수 있으므로 identifier를 바꾼다.

```bash
AW_HTTP_DIAGNOSTIC_FILE=$TMPDIR/aw-http.json AW_HTTP_WEBVIEW_PROBE_FILE=$TMPDIR/aw-probe.json \
  pnpm --filter agentic-workbench tauri dev --config '{"identifier":"com.yoophi.agentic-workbench.smoke042"}'
```

**(a) 끝점 진단 — "끝점이 인증 규칙대로 응답한다"만 증명(데스크톱 연결 증거 아님)**

1. `aw-http.json`이 생기고 권한 0600, `baseUrl`·`token`·`expiresAt`만 있다.
2. `curl $baseUrl/health/live` → `200 {"status":"live"}`(인증 없음). `/health/ready`는 토큰 없이 401.
3. 진단 토큰으로 handshake → 선택 버전 1·`serverEpoch`·`instanceId`, `POST /v1/calls` `project.list` → `CallReply`.
4. 표 발급 → 테스트 WS 클라이언트로 `hello`. 같은 표 재사용 → 401. 진단 토큰에 `Origin` 헤더를 붙이면 거절.

**(b) WebView probe — SC-005 "발급한 데스크톱 토큰·실제 Origin" 증거**

5. `aw-probe.json`에 `origin`(실제 `location.origin`)과 단계별 결과: `get_workbench_connection` 성공, handshake 200, `project.list` 200, 표 발급 200, WS `hello`, 표 재사용 거절, 토큰 없는 호출 401, 허용 목록 밖 Origin 거절은 해당 없음(브라우저가 Origin을 바꿀 수 없음 — 자동 테스트가 담당). 토큰·표 문자열은 파일에 없어야 한다.
6. 캡처한 `origin`이 허용 목록에 있어야 한다(없으면 목록을 고치고 다시).

**재기동**: 앱 종료 → 재기동 → (a)·(b) 반복(새 포트·새 `instanceId`).

스모크 뒤 격리 데이터 디렉터리와 두 파일을 지운다.

## 4. 수동 확인 (리뷰어)

- 배포(release) 빌드의 WebView Origin: probe는 debug 빌드 전용이므로, 배포 빌드의 `tauri://localhost`(macOS)는 설치본 devtools 또는 4단계 E2E에서 확인한다(자동 증거는 dev Origin까지).
- 화면 동작이 이전과 같다(이번 단계는 화면 경로를 바꾸지 않는다).
