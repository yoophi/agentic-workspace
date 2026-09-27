# 041 설계 리뷰 기록

## 1. OCR 위임 리뷰 (`/open-code-review:delegate-review`, 2026-09-27)

- 범위: `c8b41a4..b2e2264`. OCR 자동 선정 대상은 `.specify/feature.json` 1개(JSON 키 오타 규칙 — 문제 없음). `.md` 설계 문서는 OCR이 제외하므로, 위임 방식대로 host가 spec·plan·research·data-model·contracts·quickstart를 실제 코드와 대조해 검토했다(사용자 지정 점검: 저장소 전체 RMW, lock 교착, 동시 묶임, 이전 역할 권한, 종료 run 재생 수명·스트림 제거).
- 결과: High 9, Medium 6. 전부 반영.

| # | 지적 | 반영 |
|---|---|---|
| H1 | 알림 전달이 작업 영역 lock을 쥔 채 Main 턴을 기다리면 Main의 도구 호출과 교착 | R2 전면 재설계(operation 범위 lock 폐지), R9 알림 전달 3단계 |
| H2 | `waitChildTasks`가 lock 안에서 poll하면 자식 보고와 교착 | R12: lock 없이 `read` + revision watch |
| H3 | 엔진 취소가 종료 처리를 호출자 task에서 바로 부름 → 재진입 교착 | R2-3·R8: 종료 처리는 조건부 `update` 한 번, lock 대기 없음 |
| H4 | 동기 닫기 hook은 async lock을 못 잡음 | R3: 닫기 hook async화, binding mutex + `update` |
| H5 | 작업 영역 id 없는 bootstrap/recover 중복 묶기, 닫기와 경합 | R3: binding mutex 안에서 찾기·만들기·검사·삽입, 입장권 쥔 채 |
| H6 | 자식 첫 턴이 `bind_child_run` 전에 도구를 부르면 역할 없음 | R7·R8: 기동 전 Launching 기록 |
| H7 | coordinator = "마지막 세대"는 연결 해제 뒤 틀림 | R7·data-model: `activeCoordinatorGenerationId` 세대 Active |
| H8 | 복구 뒤 이전 노드 run 재생이 막혀 화면 기록 사라짐 | R17: 허용 조건 2(묶인 작업 영역의 노드 run) |
| H9 | 토큰이 run id만 가지면 `tools/list` 기준 없음, 수동 채택 자식 | R7: `orchestration.getAgentRole`, 수동 채택 자식은 도구 없음 유지 |
| M1 | `.bak` 복구 규칙이 이 저장소와 안 맞음 | R1: json_store 읽기 경로 복구를 lock 안에서 그대로, `with_aggregate` 재시도 미사용 |
| M2 | std Mutex·fsync를 async에서 직접 | R1: `spawn_blocking` |
| M3 | lock 규칙 제안 | R2 규칙 1–5, data-model 저장 경계 표 |
| M4 | 오늘 권한 오류 문구 오기 | contracts: `forbiddenActor`·`scopeMismatch` 원문 |
| M5 | 토큰 폐기 시점 오기 | tauri-compat: 재시도·재배정·교대 폐기 유지, run 종료 폐기 없음 |
| M6 | 제거된 run 응답 형태·발행 전 구독 검사 위치 | R17·contracts: Evicted 형태, seam 진입점에서 검사 |

## 2. Codex adversarial review (`/codex:adversarial-review --wait`, 2026-09-27, OCR 반영 뒤 순차 실행)

- 대상: 브랜치 diff(`01dbfca`까지). Verdict: needs-attention, High 1.

| # | 지적 | 반영 |
|---|---|---|
| C1 (high) | 재생 허용 조건 2가 호출자가 넣은 run id를 신뢰 — `bindCoordinator`·`handoffCoordinator`가 다른 작업대의 run을 검사 없이 넣으면 그 run의 기록·live 스트림이 노출되고, 같은 run이 여러 작업 영역에 들어가 역할 판정이 모호해짐(`orchestration_service.rs:365–385`, `:1484–1506`) | research R18 신설: 데스크톱 입력 run은 대상 작업대 소유의 살아 있는 run만, agent 입력 run은 principal run만, 한 run은 작업 영역 하나에만. contracts·data-model·quickstart(음성 테스트) 갱신 |

