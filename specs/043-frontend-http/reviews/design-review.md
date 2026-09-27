# 043 설계 리뷰

## 1. OCR delegate-review (`--from b682c6b --to HEAD`)

OCR 검토 대상은 `.specify/feature.json` 하나(설계 문서는 확장자로 제외) — 설계는 코드(main `b682c6b`)와 대조해 직접 검토했다. 사용자 검토 메모(R1·R7·R8·R11)를 입력에 포함했다.

| # | 등급 | 지적 | 반영 |
|---|---|---|---|
| D1 | High | R6: 브라우저 `fetch` 거절은 "보내지 못함"과 "보낸 뒤 응답 유실"을 구별하지 못한다(둘 다 `TypeError`). 연결 거절이 곧 미전송이라는 가정은 없다 | notApplied는 **클라이언트가 스스로 보내지 않은 경우**(연결 상태가 이미 `reconnecting`/`disconnected`)로만 한정. 보내기를 시도한 뒤의 모든 거절은 `unknown`(변경은 같은 세대 같은 키 재시도, 새 세대 무재전송). research R6·contracts §4 수정 |
| D2 | High | 창별 주체(R1)로 작업대를 열면, 호환 경로의 작업대 범위 command가 지금처럼 `desktop_principal()`로 부를 때 소유 판정에 걸려 **호환 경로 창이 깨진다**(`workbench_compat.rs`·`tauri_commands.rs`·`desktop_benches.rs`) | 작업대 범위 호환 command(run·교환·orchestration·`bench.*`)는 호출 창의 주체로 부른다 — 호환 경로도 창 incarnation을 쓴다. 전역 command(프로젝트 등)는 창 주체로 불러도 같다(scope 동일). 인벤토리 표에 command별 주체 열, 호환 경로 회귀 시험. plan Foundational에 명시 |
| D3 | Medium | R7: 수신자 0명 동안 큐가 무한히 자랄 수 있다 | 큐 상한(스트림당 1,024 프레임) — 넘으면 연결을 닫고 `appliedSequence`에서 다시 구독(서버 쪽 `subscriberLagged`와 같은 복구) |
| D4 | Medium | R4: 전달 끄기 표를 label로 두면 같은 label로 다시 연 호환 경로 창이 전달을 못 받는다 | 표 키를 창 incarnation으로. 창 `Destroyed`에서 제거 |
| D5 | Medium | R8(메모): 스트림별 스냅샷 규칙을 protocol과 대조해야 한다 | 확인: orchestration 이벤트 본문 `OrchestrationEventDto{workspaceId, revision, reason, taskId, nodeId}` — 상태가 아니라 변경 신호. 교환은 `ExchangeRequestedDto`(요청)·`AgentExchangeDto`(상태, `revision` 없음, `updatedAt`). 결정: orchestration = 구독(hello) → `orchestration.get` 스냅샷 → 스냅샷 `revision` 이하 이벤트 버림, 이후 이벤트는 재조회 트리거. 교환 = 구독(hello) → `exchange.list` 스냅샷 → 이벤트는 `requestId` 기준 멱등 upsert(상태는 스냅샷보다 늦은 `updatedAt`만 적용). run만 `run.replay` 기준점 방식. research R8 대응표 확정 |
| D6 | 확인 | 창별 주체가 영속 멱등을 쪼개는가 | 쪼개지 않는다 — ledger 유일 키는 `(principal_kind, operation, contract_revision, idempotency_key)`(subject 없음, `sqlite_ledger.rs`). 재시작 뒤 다른 incarnation의 같은 키 재시도도 같은 범위. 세대 멱등의 `Open` 범위는 subject 기준이라 `bench.open` 재생은 창별(의도) |
| D7 | 확인 | R1(메모): label만 subject면 같은 label 재개 창을 옛 토큰이 조작 | incarnation 포함 subject + `Destroyed`에서 토큰 폐기(research R1 반영). 시험 두 겹: 폐기 확인, 폐기를 뺀 변이에서도 incarnation 차이로 거절 |
| D8 | 확인 | R7(메모): 여러 수신자 중 일부 예외 | 수신자별 `deliveredSequence`, 콜백 동기, 예외는 넘긴 것으로 침(research R7 반영) |
| D9 | 확인 | R11(메모): 호환 기본값만으로는 FR-012 근거 부족 | 화면 시험 harness를 transport 매개변수화해 HTTP 경로에서도 같은 기대값(research R11 반영) |
| D10 | 확인 | MCP·agent 경로와 충돌 | 없음 — agent 주체·MCP 토큰은 그대로, 작업대 소유 판정은 연 주체만 본다(`close_all_benches`는 연 주체로 닫음) |
