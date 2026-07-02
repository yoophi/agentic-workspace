# Baseline / 개선 후 측정 기록

계측 코드(T001, T002)가 들어간 상태에서 quickstart.md 절차로 수집한다.
**개선 전(baseline) 수치는 Phase 2(T004) 적용 전 커밋에서 측정해야 한다** — `git stash` 또는 해당 커밋 체크아웃 후 `AW_PERF_LOG=1 pnpm --filter agentic-workbench tauri dev`.

## 측정 환경

| 항목 | 값 |
|---|---|
| 측정일 | (기입) |
| 장비 | (기입) |
| 다중 worktree 프로젝트 | (worktree 수 기입) |
| 대형 이력 저장소 | (commit 수 기입) |

## Baseline (개선 전)

| 메트릭 | 수치 | 비고 |
|---|---|---|
| `session:shell-rendered` | (ms) | quickstart S1 |
| `session:graph-first-row` | (ms) | quickstart S1 |
| 세션 진입 직후 command `wait_ms` 최대값 | (ms) | 직렬화 정도 |
| `list_git_worktrees` run_ms / 내부 git status 횟수 | (ms / 회) | |
| graph 첫 페이지 run_ms | (ms) | |
| graph 뒤 페이지(10페이지째) run_ms | (ms) | S5 |
| idle 10분 watcher git 이벤트 수 | (회) | S2, 되먹임 가설 검증 |

## 개선 후 (구현 완료 시점)

2026-07-02 dev 빌드 S1 1차 실측 (분석 방법: `docs/aw-perf-log-analysis.md`):

| 메트릭 | 수치 | 목표(SC) | 판정 |
|---|---|---|---|
| `session:shell-rendered` | (webview 콘솔 확인 필요) | < 1,000ms (SC-001) | 보류 — refs 조회 21~74ms로 backend 병목은 해소 |
| 세션 진입 직후 command `wait_ms` 최대값 | **0ms** (14.9초짜리 status 조회 진행 중에도) | 직렬 대기 소멸 (SC-002) | **통과** |
| watcher `kind=Git` 이벤트 (관찰 구간) | **0회** (File 10회) | 0회 (SC-003) | 예비 통과 — 10분 idle 관찰로 확정 |
| graph view 진입 시 history 조회 | **0회** (graph 1회 300ms) | 미실행 (SC-005) | **통과** |
| graph 뒤 페이지 run_ms / 첫 페이지 run_ms | (대형 저장소에서 측정 필요) | ≤ 2배 (SC-006) | 보류 |
| 1,000+ row 로드 후 DOM row 수 | (측정 필요, Storybook `VirtualizedHistoryGraphLargeRepo` 참고) | viewport 수준 (SC-007) | 보류 |

추가 실측 발견: 메인 창의 3초 interval status 폴링이 사이클당 worktree당 평균 739ms(최대 7.8초, 목록 전체 최대 14.9초)의 git 작업을 지속 발생시킨다 — 조사 문서 P2 항목의 후속 개선(interval 완화/focus 기반) 근거.
