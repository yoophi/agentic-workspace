# 044 설계 리뷰

## OCR 설계 리뷰 (`ocr delegate preview --from cb0bd4c --to 5f9680e`)

OCR은 `.md`를 검토 대상에서 뺀다(10개 중 `.specify/feature.json` 1개만 대상). 그래서 043 설계 리뷰와 같이 **설계 문서를 실제 코드와 직접 대조해 검토했다**. 기준 main `cb0bd4c`.

| # | 등급 | 문제 | 반영 |
|---|---|---|---|
| D1 | High | `ensure`가 남은 안내 파일의 끝점에 곧바로 소유자 자격 증명을 보낸다. 원래 서버가 죽고 그 포트를 다른 프로세스가 차지했다면 자격 증명이 새어 나간다 | 인증 없는 `/v1/system/identify`(nonce → HMAC 증명)로 신원을 먼저 확인하고, 맞을 때만 자격 증명을 보낸다(research R5, lifecycle §3·§4) |
| D2 | High | "소유자는 작업대 소유 판정 우회"만 적고 우회 지점을 정하지 않았다. 소유 판정은 레지스트리의 주체 비교(`bench_service.rs:166-185`)와 이벤트 hub 스트림 claim 두 곳이다. 하나만 바꾸면 조회는 되고 구독은 안 되는 식으로 어긋난다. agent 전용 operation까지 우회하면 안 된다 | 두 지점을 명시하고 각각 시험한다. agent 전용 operation은 우회 제외(lifecycle §4) |
| D3 | Medium | R7-check(대기 자식 명령 전달 경로)가 미정이었다 | 코드 판독 결과를 적었다. 대기 명령은 엔진 대기열(`engine_agent_worker.rs:197`), 대기 task는 스케줄러 `release`(`scheduler.rs:56`). 둘 다 서버 내부라 추가 K는 없다. 대신 활성 작업에 센다(R7) |
| D4 | Medium | embedded 모드가 소유 잠금만 잡고 안내 파일을 쓰지 않으면, 다른 클라이언트의 `ensure`가 20초를 헛기다리다 원인 모를 실패를 한다 | embedded도 안내 파일(`mode: "embedded"`)을 쓴다(desktop §1) |
| D5 | Medium | 활성 작업의 "권한 대기 수"는 셀 수단이 없다(`PermissionBroker`에 수 조회 없음, `acp-agent-core` 변경 필요). 대기 task 수도 빠져 있었다 | 권한 대기 run은 활성 run에 포함되므로 따로 막지 않는다(`active_run_count`). 스케줄러 대기 task 수를 더했다(R7) |
| D6 | Low→기록 | 데스크톱이 띄운 서버는 데스크톱 환경 변수를 물려받는다(agent 카탈로그가 환경 변수를 읽음). 오늘과 같지만, 6단계 CLI가 서버를 띄우면 환경이 달라진다 | 이 증분은 오늘과 같음. 6단계 추적 항목 |

사용자 검토(R7 실제 전달 경로·ledger `unknown`, R8 순서 주장)는 research R7·R8에 반영돼 있다: K 이어 가기 계약, 실제 경로 wait-stop 시험과 대조 변이, `unknown` 비차단, R8-spike 전 순서 무관 주장 금지, 관측 경로마다 구현·실검증이 완료 조건.
