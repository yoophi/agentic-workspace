# Quickstart: 040 검증 절차

## 1. 자동 게이트

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
pnpm run check-types
pnpm run test
pnpm run generate:contracts && git status --short   # 생성물 diff 0
```

기대:

- call fixture(작업대·run·교환, `steps` 치환 포함)와 이벤트 fixture(교환·작업대 스트림)가 in-memory·테스트 HTTP 두 경로에서 같은 결과.
- 흐름 테스트: run 수명, 교차 작업대·교차 주체 거절(run 제어 6종 + 교환), 교환 두 작업대 격리·확인 멱등, 작업대 닫기 → 소유 run 취소·스트림 `Gap(evicted)`, 세대 범위 멱등(재시도 같은 결과·다른 payload `conflict`·동시 요청 한 번 실행), `run.start` 재시작 판정(`pending` → `unknown`).
- 기존 AW run·교환·MCP 테스트는 기대값 수정 없이 통과(교차 작업대 거절처럼 의도적으로 바뀐 것은 tasks Notes에 기록).

## 2. 경계 확인

```bash
git diff --stat main -- crates/acp-agent-core packages/agent-client   # 0
git diff --name-only main -- apps/agentic-workbench/src                # 0 (화면 불변 목표)
grep -rn "window_label\|windowLabel" crates/workbench-protocol crates/workbench-core/src | grep -v "^.*test"   # 0 (SC-002)
```

## 3. 앱 수동 확인

`pnpm --filter agentic-workbench tauri dev`:

| # | 조작 | 기대 |
|---|---|---|
| 1 | 세션 창에서 run 시작 → 프롬프트 → 권한 요청 응답 → 조향/취소 후 전송 → 취소 | 이전과 같은 표시·문구 |
| 2 | 같은 Worktree를 세션 창 두 개로 열고 한 창에서 두 패널 사이 교환 | 그 창만 요청을 받아 한 번 라우팅·확인, 다른 창은 변화 없음 |
| 3 | agent가 `set_window_title` 호출(두 창 열린 상태) | 그 run의 창 제목만 바뀜 |
| 4 | run 진행 중 세션 창 닫기 | run 취소(이전과 같음), 다른 창 영향 없음 |
| 5 | orchestration workspace 시작·자식 위임(041 전 과도기) | 이전과 같이 동작, 창 닫으면 자식 run도 취소 |
| 6 | 앱 재시작 뒤 세션 창 복원 | 이전과 같은 표시(이전 run은 유실 표시) |
