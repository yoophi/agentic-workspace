# Quickstart: 043 검증

## 1. 게이트(각 한 번, 원 명령 종료 코드 기록)

```bash
cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo test --workspace --all-targets --no-fail-fast
pnpm run check-types
pnpm run test
```

## 2. 핵심 시험

- `packages/workbench-client`: 세 호출 결과·같은 세대 재시도·새 세대 무재전송, 반영 완료 cursor·수신자 교체·예외, gap 사유별 순서, 강제 끊김 100회(누락·중복 0).
- AW 저장소 동등성: 모든 저장소 함수를 두 transport로 같은 결과·오류 문자열.
- `crates/workbench-core/tests/window_isolation.rs`: 다른 창 주체의 조작·구독 거절.

## 3. 실제 앱 스모크(debug, 격리 identifier)

```bash
AW_HTTP_WEBVIEW_PROBE_FILE=$TMPDIR/aw-probe.json \
  pnpm --filter agentic-workbench tauri dev --config '{"identifier":"com.yoophi.agentic-workbench.smoke043"}'
```

probe(앱 자신의 transport 사용): 메인 창 경로 = `http`, 프로젝트 조회, 세션 창 열기 → `ensure_window_bench` → run 시작(가짜 ACP agent, `agentCommand`) → 출력 구독 수신 → debug hook으로 서버가 구독을 닫음 → 자동 재연결·이어 받기(순번 연속) → 앱 내부 fallback 이벤트 0건 → 다른 창 토큰으로 이 작업대 조작 거절. 배포 frontend(`tauri build --debug --no-bundle`, `tauri://localhost`)로 반복. 끝점 기동 실패 주입(debug env) → 창 경로 `compat`, 주요 흐름 동작.

종료는 `NSRunningApplication.terminate`(`pgrep -f "^<경로>"`로 앱 PID 지정). 스모크 뒤 격리 데이터 삭제.

## 4. 수동(리뷰어)

세션 창 두 개에서 run 출력이 섞이지 않는지, 창 닫기로 그 창 run만 끝나는지, 연결 상태 표시.
