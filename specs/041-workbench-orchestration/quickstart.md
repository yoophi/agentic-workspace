# Quickstart: 041 검증 절차

작업 위치: `/Users/yoophi/project/worktrees/041-workbench-orchestration`. 디스크 여유가 작으면 `CARGO_INCREMENTAL=0`.

## 1. 자동 검증

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
(cd apps/agentic-workbench/src-tauri && cargo check --release)
pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated
pnpm run check-types
pnpm run test
```

기대:

- 계약 suite(`crates/workbench-core/tests/contract_suite.rs`)가 orchestration fixture(데스크톱 18·agent 17, 거절 시나리오)를 메모리·HTTP 두 경로에서 같은 결과로 통과한다([contracts/workbench-orchestration.md](contracts/workbench-orchestration.md)).
- 동시성 테스트(research R1): 서로 다른 작업 영역 2개 이상에 동시 변경 100회 이상, 같은 작업 영역에 화면 명령 + agent 보고 동시 100회 이상 → 잃어버린 변경 0. 저장소 lock을 빼면 실패함을 한 번 확인한다.
- 스트림 테스트: 묶임 스트림 순번, 다른 주체·agent 구독 거절, 작업대 닫기 → gap, 재묶임 → 새 `eventStreamId`. run 스트림 구독·`run.replay`가 다른 작업대 주체에게 거절된다(research R17).
- 흐름 테스트: 작업대 닫기 → 복구 가능 → 다른 작업대로 `orchestration.recover`, 자식 run이 worktree를 바꾸고 끝나면 과제 실패, scheduler 한도 초과 과제가 대기 후 진행, 이전 세대 coordinator의 agent operation 거절.
- 출처 검증 음성 테스트(research R18): 다른 작업대의 run으로 `bindCoordinator`·`handoffCoordinator` → `forbidden`·상태 불변, 이어서 그 run의 `run.replay`·`run:<id>` 구독도 거절. 같은 run을 두 작업 영역에 넣으면 `conflict`.
- liveness 테스트(설계 리뷰 H1–H4 회귀 방지, 각각 제한 시간 안에 끝나야 함): ① coordinator 알림 전달이 Main 턴을 기다리는 동안 Main이 수집·대기 도구를 부른다, ② `waitChildTasks` 대기 중 자식이 결과를 보고한다, ③ 과제 취소가 엔진 취소 → 종료 처리를 같은 task 안에서 부른다, ④ 자식 기동 중 작업대를 닫는다, ⑤ 두 작업대가 같은 worktree에서 동시에 bootstrap·recover한다(하나만 묶임).
- describe fixture: 데스크톱 85, readonly(조회만), agent(허용분만).

## 2. 경계 확인

```bash
git diff --stat origin/main -- crates/acp-agent-core packages/agent-client apps/agentic-workbench/src   # 0
grep -rniE "window_?label|windowLabel|boundWindowLabel" crates/workbench-core/src crates/workbench-protocol/src   # parity 단정 테스트만
grep -rn "acp_registry\|\.run_sink(\|resolve_agent_run_launch_principal\|release_window" apps/agentic-workbench/src-tauri/src   # 0
```

## 3. 앱 수동 확인 (리뷰어)

`pnpm --filter agentic-workbench tauri dev`:

| # | 조작 | 기대 |
|---|---|---|
| 1 | 세션 창에서 orchestration 시작 → Main run 시작·연결 → 목표 위임 | 이전과 같은 표시, coordinator가 자식 과제를 만든다 |
| 2 | 자식 과제 취소·재시도·재배정, 프롬프트 분배 | 이전과 같은 표시·문구 |
| 3 | 세션 창 두 개(다른 worktree)에서 각각 orchestration 진행 | 한 쪽 갱신이 다른 창에 나타나지 않음 |
| 4 | orchestration 진행 중 창 닫기 → 같은 worktree 다시 열기 → 복구 | 자식 run 취소, 복구 목록에 보이고 복구됨 |
| 5 | 앱 재시작 뒤 같은 worktree 열기 | 작업 영역이 복구 가능으로 보이고 복구됨 |
