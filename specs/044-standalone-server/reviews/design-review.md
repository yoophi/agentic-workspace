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

## Codex 적대적 설계 리뷰 (`--wait --base cb0bd4c`, 대상 `00654e9`)

판정: needs-attention. High 5건(문서·코드 대조, 실행 검증 없음). 모두 코드 판독으로 확인했다.

| # | 문제 | 근거(Codex·재확인) | 반영 |
|---|---|---|---|
| C1 | R7-check의 "대기 task는 서버가 띄운다"는 틀렸다. 스케줄러 `release`는 다음 준비 task id를 돌려줄 뿐이고, coordinator가 `assignChildTask`를 다시 불러야 한다. 이를 N으로 막고 대기 task를 활성 작업에 세면 wait-stop이 멈추지 않는다 | `agent_tools.rs:555-570`, `orchestration_liveness.rs:232-266` | `assignChildTask`를 K로 둔다(비우기 시작 전 대기 task만, 상태 전이로 1회). 동시 실행 상한 1 실제 경로 시험 |
| C2 | 043은 prompt 전송 **전에** 확인한다(라우팅 = 상태 갱신, 전송은 turn 뒤 effect). 교환 상태(`accepted`)로 K를 판정하면 확인 뒤라 거절된다 | `exchange-reconciler.ts:45-55`, `worktree-agent-run-area.tsx:315-337`, `agent-run-panel.tsx:1042-1064·1458-1471` | 교환 상태 대신 서버가 관리하는 **전달 prompt 소비 여부**로 판정한다. 확인 결과 `rejected`만 제외하고 `draft`는 K가 아니다. 확인이 전송보다 먼저 오는 실제 화면 경로 시험 |
| C3 | 세션 수(`active_run_count` = `runs.len()`)는 turn이 끝난 뒤 다음 prompt를 기다리는 ACP 프로세스도 센다. wait·유휴가 끝나지 않는다. accessor도 시험 전용이다 | `runner.rs:419-475`, `start_agent_run.rs:87-88` | 활성 작업은 **바쁜 run**(진행 중 turn·엔진 대기열 prompt·권한 대기)으로 센다. core가 run 이벤트로 운영용 `RunActivity`를 유지한다. 쉬는 세션은 정지 때 취소한다. prompt 완료 뒤 살아 있는 ACP 프로세스로 시험 |
| C4 | 등록 없이 요청한 incarnation에 토큰을 발급하면, 폐기보다 늦게 도착한 발급이 폐기된 창의 토큰을 되살린다. `revoke_subject`는 현재 토큰만 지운다 | `auth.rs:127-132` | 세대 동안 폐기 tombstone. 발급·폐기를 같은 잠금으로 직렬화하고 tombstone 주체에는 발급하지 않는다. 역순·새 incarnation·동시 시험 |
| C5 | K 조건이 교환을 소비하지 않아, 한 교환 id로 다른 키·다른 내용의 prompt를 계속 보낼 수 있다. 유한성 논증이 깨진다 | `epoch_idempotency.rs`(키별 중복 제거뿐) | 교환마다 전달 prompt **1회 소비**(원자적), 키는 `exchange-delivery:<id>`로 고정. 둘째 prompt·동시 요청은 N. 내용 결합은 하지 않는다(비우기는 보안 경계가 아님, 유한성에는 1회 소비로 충분) |

사용자 요청에 따라 수정된 설계의 이 5건을 Codex `--wait`로 다시 검토한다(아래).
