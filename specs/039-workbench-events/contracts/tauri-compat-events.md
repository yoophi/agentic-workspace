# Contract: 데스크톱 호환 (039)

프론트 동작은 바뀌지 않는다. 바뀌는 프론트 코드는 run 화면의 순번 처리뿐이다(spec Q2).

## run 이벤트 전달

| 항목 | 이전 | 이후 |
|---|---|---|
| 경로 | Tauri `agent-run-event`(전체 방송) + 창 삽입 `agent-run-event-fallback` | 창 삽입 `agent-run-event-fallback`만 |
| payload | `{runId, event}` | `{runId, event, sequence, epoch, streamId, eventId}`(상위집합) |
| 대상 창 | sink 생성 시 label | 같음(ADR 0003) |
| 창 없음 | `emit_to(label)`(listener 없음) | 전달하지 않음 |
| journal | AW `InMemoryRuntimeEventJournal` append | core hub `publish_run` |
| 전달 순서 | 발행자마다 따로 전달(동시 발행 시 순서 보장 없음) | 순번 부여와 같은 스트림 lock 안에서 `eval` → 창 도착 순서 = 순번 순서(여러 발행자·sink 인스턴스여도) |

## run replay command (2b 이연 대상이지만 구현만 교체)

`replay_orchestration_runtime_events({runId, afterSequence})` → `RuntimeEventSnapshot {runId, events[{runId, sequence, event, terminal}], lastSequence, terminal, gapDetected}` — 형태·의미 불변, 데이터 출처만 hub. **새 경우 하나**: 보관 run 상한으로 제거된 run은 `{events: [], lastSequence: 0, terminal: true, gapDetected: true}`를 돌려주어 화면이 `gap`을 표시한다(오늘은 journal을 지우지 않아 이 경우가 없었다). 이전 세대(재시작 전) run을 cursor 0으로 재수화하면 오늘처럼 빈 snapshot이다.

## worktree 감시 command

| command | 이후 |
|---|---|
| `start_worktree_watcher(window, workingDirectory)` | `Workbench.events(desktop, [worktree:<wd>])` 구독 task. 기존 task 있으면 교체. 이벤트마다 `workingDirectory`를 호출자 문자열로 바꿔 `emit_to(label, "workspace://worktree-changed")` |
| `stop_worktree_watcher(window)` | 그 창 task abort(구독 해제) |
| 창 파괴 | 같음(`stop_for_window`) |
| 오류 | 오늘 문구 그대로(`Cannot watch missing worktree path: …`) |

## 바뀌지 않는 것

orchestration·exchange·창 제목·외관 설정 이벤트 경로, 창을 닫으면 run 취소, run·orchestration command(2b).
