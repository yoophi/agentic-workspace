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

## Codex 설계 재검토 1 (`--wait --base 00654e9`, 대상 `54b31a8`, 수정된 5건 집중)

판정: needs-attention. C4(폐기 tombstone)는 타당하다고 확인했다. C1·C3·C5의 수정에 High 3건:

| # | 문제 | 근거 | 반영 |
|---|---|---|---|
| E1 | run 이벤트만으로는 활동을 정확히 셀 수 없다. 대기열 등록 이벤트가 없고, RPC 오류는 `PromptCompleted` 없이 끝나며, 시작 중·Ralph 반복 사이가 드러나지 않는다 | `acp_run_engine.rs:125-148`, `runner.rs:748·794-799` | 실행 수명 계약: 진입점에서 동기 예약, 실행 future 종료에서 해제. 초기 prompt 순서는 acp-agent-core runner가 순서 끝에서 guard를 놓는다(선택 인자, ask-code·hushline 검증). 정지 판정과 예약을 한 잠금으로 직렬화 |
| E2 | 소비를 `run.sendPrompt` 성공에 묶으면, 전송이 spawn 뒤 바쁨으로 실패해도 소비·멱등 성공이 남아 교환을 잃는다 | `send_prompt.rs:46-58`, `runner.rs:787-790` | `continuation` prompt는 엔진 대기열 경로로 보내고, 소비 표시와 대기열 등록을 한 번에 한다. 활동 예약이 전달 끝까지 남는다. 알림 prompt와 경합하는 실제 경로 시험 |
| E3 | 배정은 원자적이지 않다(동시 배정이면 run 둘, `reserve_child_run`이 덮어씀). K의 "상태 전이로 1회"가 성립하지 않는다. 기존 결함이다 | `agent_tools.rs` assign, `service.rs:802-828` | 저장소 단일 RMW 경계 안에서 비교 후 변경(`Ready`·예약 없음 → `Starting`). 기존 예약이면 그 결과를 돌려준다. 동시 배정·취소 경합 시험 |

## Codex 설계 재검토 2 (`--wait --base 54b31a8`, 대상 `7902e15`)

판정: needs-attention. E1의 실행 guard와 E2의 엔진 대기열 전환은 원인에 직접 대응한다고 확인했다. High 1건:

| # | 문제 | 근거 | 반영 |
|---|---|---|---|
| E4 | 저장소 비교 후 변경은 중복 배정만 막는다. `Starting` 예약 뒤·엔진 등록 전(`fingerprint_worktree` 대기)에 취소하면, 취소는 accepted로 끝나고 task가 `Cancelled`가 된다. 그런데 기동 경로는 취소를 확인하지 않고 run을 띄우며, `bind_child_run`도 취소된 task를 거절하지 않는다 | `agent_tools.rs:798-824`, `engine_agent_worker.rs:155-159` | 기동 토큰(`Pending→Registered|Cancelled|Failed`)으로 취소와 엔진 등록의 인계를 G 아래에서 직렬화한다. 등록 전 취소는 기동을 막고, 등록이 먼저면 실제 run을 취소한다. `bind_child_run`은 취소된 task를 거절한다. 등록 직전 gate 결정적 시험 |

**사용자 검토 5 반영**: 개별 조건을 덧붙이지 않고, 활동 예약·교환 전달 수락·task 기동 예약·정지 판정을 한 경계(작업 관문 G)와 한 상태 전이 표로 정리했다(research R14). 표는 성공·오류·취소 해제와, 닫는 실패 순서·검증을 함께 적는다.

아직 데스크톱 UI에 의존하는 교환 전달(창 원장 라우팅·패널 대기열)은 plan·spec의 후속 미완료 표에 적었다. 5단계 (a) 완료로 세지 않는다.

## Codex 설계 재검토 3 (`--wait --base 7902e15`, 대상 `5ce3744`)

판정: needs-attention. UI 의존 교환 전달의 미완료 표기는 정확하다고 확인했다. High 2건:

| # | 문제 | 근거 | 반영 |
|---|---|---|---|
| F1 | 비동기 엔진 등록(`reserve_run` → spawn → `attach_run_handle`)과 `Pending→Registered` 사이에 선형화 지점이 없다. 등록 뒤에 전이하면 그 사이 취소가 spawn한 실행을 못 막고, 등록 전에 전이하면 없는 run을 취소하고 끝난다 | `start_agent_run.rs`, `AppState::cancel_run` | 엔진 시작을 준비(예약·장벽에서 기다리는 spawn·attach)와 실행 허용으로 나눈다(`acp-agent-core` 선택 인자 `start_gate`). G 아래 전이를 선형화 지점으로 삼는다. 지점별 결정적 취소·abort 시험 |
| F2 | 자식 보고는 결과를 저장하고 전달기를 spawn한 뒤 돌아간다. 전달기 첫 poll 전에 활동이 0이 되어 정지하면 알림을 잃는다 | `agent_tools.rs:540-570` | 저장된 미전달 알림(대상 coordinator 살아 있음)을 활동에 센다. 보고 C-call 해제 전 N-notify 예약, 비우기 진입·재시도 실패 뒤 서버가 전달 한 바퀴. 전달기 첫 poll gate 시험 |

## Codex 설계 재검토 4 (`--wait --base 5ce3744`, 대상 `dd79a23`)

판정: needs-attention. F1의 시작 장벽은 등록 경쟁을 해소한다고 확인했다. High 1건:

| # | 문제 | 근거 | 반영 |
|---|---|---|---|
| G1 | 중단된 `Dispatching` 알림을 다시 전달 가능한 상태로 되돌리는 전이가 없다. 전달기는 `Pending`·재시도 가능 `Failed`만 고르므로, `Dispatching` 저장 뒤 drop이나 결과 저장 실패가 나면 활동으로만 남아 wait가 끝나지 않는다 | `notification_dispatcher.rs`, `recover_interrupted`는 재시작 경로 | 전달 시도 `attemptId` 소유권을 둔다. 예약 없이 남은 같은 시도의 `Dispatching`만 `Failed(retryable)`로 회수하고 서버가 다시 전달한다(6''). abort·결과 저장 실패 주입 시험, 정상 시도 비회수 시험 |

## Codex 설계 재검토 5 (`--wait --base dd79a23`, 대상 `4d6d3ff`)

판정: needs-attention. High 1건:

| # | 문제 | 근거 | 반영 |
|---|---|---|---|
| G2 | 6'는 N-notify를 `send_and_wait` 시작 때 A-turn으로 인계하는데, 2행은 A-turn을 실행 future 끝에 놓는다. 전달기는 `notify_coordinator().await` 뒤 별도 transaction으로 결과를 저장하므로, prompt 완료 뒤·결과 저장 전에 `Dispatching`만 있고 예약은 없는 구간이 생긴다. 그때 회수가 돌면 정상 시도를 되돌리고 coordinator turn을 한 번 더 만든다 | `notification_dispatcher.rs` | N-notify를 결과 저장 commit까지 A-turn과 별개로 유지한다(인계하지 않음). 회수 조건은 N-notify 예약 유무만 본다. 결과 transaction 직전 gate 시험(회수 → 변경 0, abort → 회수) |

## Codex 설계 재검토 6 (`--wait --base 4d6d3ff`, 대상 `7cc3202`)

판정: needs-attention(Medium 1건, High 없음). G2의 "결과 commit까지 독립 예약" 방식은 타당하다고 확인했다. X→A·T→A 인계에서는 같은 해제 공백을 찾지 못했다.

| # | 등급 | 문제 | 반영 |
|---|---|---|---|
| G3 | Medium | 알림 전달 절 본문에 "`send_and_wait`에 들어가면 A-turn으로 인계"라는 옛 지시가 남아 예약 정의·6'행과 모순된다 | 문장을 고쳐 "N-notify는 결과 commit까지 유지, `send_and_wait`는 별도 A-turn"으로 통일했다 |

설계 리뷰 종결: 재검토 6의 남은 지적은 문서 불일치 하나였다. 이를 고친 뒤 High 지적 없이 tasks로 진행한다. 구현 리뷰(OCR → Codex)에서 R14 표의 각 칸과 결정적 시험을 다시 대조한다.
