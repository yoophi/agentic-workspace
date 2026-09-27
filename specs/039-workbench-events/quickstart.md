# Quickstart: 039 검증 절차

worktree `/Users/yoophi/project/worktrees/039-workbench-events`, 선행: `pnpm install --frozen-lockfile`.

## 1. 자동 검증

```bash
cargo test -p workbench-protocol -p workbench-core -p agentic-workbench
cargo test -p workbench-core --test event_subscription_race      # SC-001: 1,000회 누락·중복 0
cargo test -p workbench-core --test run_delivery_order           # 동시 발행자에서 창 전달 순서 = 순번 순서(1,000회)
cargo test -p workbench-core --test event_contract_suite         # SC-003·SC-004: in-memory·WS 공통 fixture
cargo test -p workbench-core --test worktree_stream              # SC-005: 감시 1회 시작·1회 중지
pnpm --filter agentic-workbench test -- agent-run-controller     # SC-002: 재수화 중 live 버퍼링 6건(replay 응답 전 live 11 도착 → 1–11 모두 반영 포함)
pnpm --filter @yoophi/workbench-client check-types test          # SC-007: EventMap 상관 타입
pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated
cargo test -p workbench-core --test event_latency -- --ignored --nocapture   # SC-006
```

## 2. 경계 확인(SC-008)

```bash
git diff --stat origin/main -- apps/agentic-workbench/src | grep -v -E "features/agent-run|entities/agent-run"   # 비어 있음
git diff --stat origin/main -- crates/acp-agent-core packages/agent-client                                         # 비어 있음
grep -rn "InMemoryRuntimeEventJournal\|fs_worktree_watcher\|AGENT_RUN_EVENT" apps/agentic-workbench/src-tauri/src  # 없음
```

## 3. 앱 수동 확인

`pnpm --filter agentic-workbench tauri dev`:

| # | 조작 | 기대 |
|---|---|---|
| 1 | orchestration 화면에서 child agent run이 빠르게 메시지를 내는 동안 창 새로고침(재수화) | 재수화 응답보다 먼저 도착한 메시지까지 포함해 앞선 메시지가 사라지거나 두 번 보이지 않음 |
| 2 | 세션 창에서 run 실행 | 진행·권한 요청·완료 표시가 이전과 같음. 다른 세션 창에는 이벤트가 보이지 않음 |
| 3 | worktree 창 두 개로 같은 worktree 열고 파일 수정 | 두 창 모두 목록 갱신, 한 창 닫아도 다른 창 계속 갱신 |
| 4 | worktree 창을 모두 닫음 | 감시 종료(`AW_PERF_LOG=1`로 watcher 로그가 멈춤) |
| 5 | 앱 재시작 뒤 이전 run 화면 복원 | 이전 버전과 같은 표시(재시작 전 run의 유실 확정은 2b) |
