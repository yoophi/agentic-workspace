# 구현 리뷰 1 — OpenCodeReview delegate (041)

- 실행: `ocr delegate preview --from c8b41a4 --to HEAD`(reviewable 78 / 155 files, +26,626/−5,878) → `ocr delegate rule`(Rust 규칙: 동시성·async 취소 안전·성능·API 설계·보안) → range diff 검토.
- 초점(사용자 요청): R18(작업대 닫기·복구·재연결 포함), claim 경합, lock 교착, 스트림 수명.

## 발견과 처리

| # | 등급 | 내용 | 처리 |
|---|---|---|---|
| O1 | Medium | `WorkbenchRunSink::emit`이 run 이벤트마다 `claim_run`(retention mutex + stream id 문자열 할당)을 불렀다 — 토큰 단위 스트리밍 hot path | 첫 발행(`sequence == 1`)에서만 claim. 기동 경로(`run.start`·자식 기동)는 엔진 호출 전에 이미 claim하므로 권한 판단은 바뀌지 않는다 |
| O2 | Medium | compat 오류 문자열을 `details.orchestrationError` Value에서 다시 만든다 — 오늘 `serde_json::to_string(&OrchestrationError)`와 바이트가 같은지 고정한 테스트가 없었다(화면이 JSON으로 파싱) | AW 단위 테스트 `orchestration_fault_string_is_byte_identical_to_the_domain_error_json` 추가(평문 fault는 문구 그대로) |

## 확인했고 결함이 아닌 것

- **재진입 교착**: 이벤트 sink가 binding mutex를 잡으므로 transaction 중 emit이 있으면 교착이다 — 서비스·명령·알림의 emit은 모두 `tx.commit()` 뒤(guard 소멸 뒤)다(`service.rs` emit 지점 전수 확인).
- **lock 순서**: binding mutex → 저장 경계만 존재. 묶임 observer(`hub.remove_stream`)는 commit 안에서 hub만 만진다(hub는 binding을 부르지 않음).
- **agent command 멱등 scope**: `Scope::RunOwner`는 소유 작업대가 없으면 기록 없이 실행한다 — 작업대 닫힌 뒤 도구 호출은 `scopeMismatch`로 끝난다(fixture `orchestration-agent-unbound-workspace-scope-mismatch`).
- **R18 닫기·복구·재연결**: 끝난 run id 재사용 거절은 hub 발행 이력(보관 중·제거 표식)과 작업 영역 기록 두 근거라, 보관 한도로 표식이 사라져도 작업 영역이 기록 중이면 거절된다. 계획 id claim은 다른 작업대의 시작·묶기를 엔진 호출 전에 막는다(60회 경합 테스트, 변이 확인).
- **스트림 수명**: 묶임 해제 commit에서 스트림 제거 → 구독자 `Gap(evicted)`, 재묶임은 새 id. 발행은 저장 뒤·mutex 밖이라 그 사이 묶임이 바뀌면 버린다.
- **발행 순서**: 두 변경의 발행 순서가 revision 순서와 뒤바뀔 수 있다(발행이 commit 뒤·lock 밖) — 오늘 AW도 저장 뒤 lock 없이 emit했고 화면은 이벤트마다 다시 읽는다. 행동 변경 아님.

## 알려진 한계(기록)

- 계획 id로 묶었지만 끝내 시작하지 않은 run의 claim은 hub 소유 표에 남는다(발행 이력이 없어 보관 한도 정리에 걸리지 않음). 같은 작업대의 재시작 재시도를 위해 의도적으로 두었다. 크기는 묶기 횟수에 비례하고 서버 재시작 때 사라진다.

---

# 구현 리뷰 2 — Codex adversarial review (041)

## 실제 범위와 제외

- 첫 실행 `adversarial-review --wait --base c8b41a4 …`는 companion이 전체 diff를 git 출력 버퍼에 담지 못해 `spawnSync git ENOBUFS`로 실패했다(diff +26,626줄).
- 재실행 범위: 같은 worktree의 임시 브랜치 `codex-review-041-tmp`에서 **생성물만 기준 상태로 되돌린 커밋**(61파일 = orchestration 계약 fixture 53 + 이벤트 fixture 3(추가 2·삭제 1) + describe fixture 3 + OpenAPI golden 1 + 생성 TS 1) 위에서 `--base c8b41a4` — 96파일 +9,995/−3,856(코드·테스트·문서 전부). 리뷰 뒤 원래 브랜치로 돌아와 임시 브랜치를 지웠고, 그 커밋은 PR에 없다.
- **제외 범위의 한계**: 제외한 fixture에는 권한·역할·재연결 동작의 golden 기대값이 들어 있다. Codex는 그 기대값을 보지 않았다 — 아래 "보완 검토"로 따로 리뷰했다.

## 발견(판정 needs-attention, high 4)과 처리

| # | 내용 | 처리 | 회귀 테스트(변이 확인) |
|---|---|---|---|
| C1 | 작업대를 닫고 다른 작업대가 재개하면 끝난 이전 run(폐기 안 된 토큰)이 역할을 되찾음 | 역할에 "지금 묶인 작업대가 살아 있는 run으로 소유"(엔진 `active_owner_of`)를 요구(기동 중 자식 예외), 닫기가 취소한 run의 토큰 폐기 | `previous_runs_do_not_regain_roles_after_another_bench_resumes` — 역할 null·도구 `forbiddenActor`·revision·과제 불변·토큰 폐기 기록. 살아 있는 소유 검사를 끄면 역할이 `"coordinator"`로 나와 실패 |
| C2 | 첫 턴 결과·입력 요청이 성공 응답만 받고 과제에 반영되지 않음(`is_current_run` false) | 엔진 호출 전에 예정 run을 노드 현재 run으로 예약(`reserve_child_run`), 첫 보고가 Ready→Running 뒤 반영, 기동 실패 시 예약 되돌림, `bind_child_run`은 첫 턴에 끝난 상태를 되돌리지 않음 | `first_turn_result_and_input_request_update_the_task_and_notify` — 결과: 과제 `completed`·노드 `idle`·result 알림, 입력 요청: `inputRequired`·노드 `active`·inputRequest 알림. 예약을 끄면 과제가 `"running"`으로 남아 실패 |
| C3 | 이전 run의 늦은 종료 검사가 새 시도를 실패 처리 | 감시에 시도 번호·run id, `fail_task_for_runtime`이 둘 다 대조 | 서비스 단위 `late_runtime_failure_of_a_previous_attempt_does_not_touch_the_new_attempt` — 이전 시도·run의 실패 반영은 거절되고 과제는 `Running`·실패 없음, 현재 시도·run이면 `Failed`. 대조를 끄면(`if false && …`) `stale.is_err()`에서 실패(변이 확인은 PR 작성 뒤 사용자 점검으로 추가 수행 — 처음 PR 문구는 이 검증 전에 "네 건 변이 확인"이라고 적어 부정확했다) |
| C4 | 닫기 중 저장 실패를 숨겨 작업 영역을 복구 불가로 남김 | 저장 실패 시 메모리 묶임 해제 + 로그. `bench.close`의 `closed: true`는 유지(근거: research R3 "닫기 저장 실패") | `a_failed_release_write_still_leaves_the_workspace_resumable` — 쓰기 금지로 저장 실패 → 다른 작업대 복구 목록에 즉시 보임 → 쓰기 복구 뒤 재시작 없이 재개. 메모리 해제를 끄면 복구 목록 `[]`로 실패 |

## 보완 검토 — 제외한 fixture의 기대값(실행 성공과 별개)

orchestration fixture 53개의 단계별 기대(결과·오류 코드·문구·details)를 추출해(`scratchpad/fixture-expectations.txt`) contracts와 대조했다.

- **약점 발견**: 거절 fixture 다수가 오류 코드만 단정하고 오늘 문구를 단정하지 않았고(`forbiddenActor`·`scopeMismatch`·run 불일치), 정상 fixture 다수가 `ok()`만 보고 상태 변화를 단정하지 않았다.
- **보강**: 거절은 문구와 `toolError{code, message, retryable}` 전체(`"The authenticated agent role cannot call this tool."`, `"This run is not bound to an orchestration workspace."`, `"The requested run does not match the authenticated capability."`). 정상은 상태: 취소 → 과제 `cancelled`, 재시도 → `attempt: 2`, 재배정 → `assignedNodeId`, 목표 위임 → 루트 과제가 활성 세대 목록에 보임(`rootTaskId` capture로 존재 확인), 자식 보고 → 보고 `type`·`progressPercent`·`reporterRunId`(principal run)와 이어지는 과제 상태(`completed`·`inputRequired`·`blocked` — 해당 알림이 `delivered`가 된 뒤 조건 대기로 읽음), 명령 → `kind`·`status: accepted`, 대기·수집 → 과제 `completed`·결과 보고.
- 보강 중 기대값이 틀렸던 곳 1건: 목표 위임 뒤 과제 목록을 빈 목록으로 적었다가 실제로는 루트 과제가 생긴다는 계약(contracts `delegateGoal`)에 맞춰 고쳤다.
- 경로 비교에서 뺀 값은 두 가지이고 fixture에 이유를 적었다: 보고 응답의 `notifications`(백그라운드 전달과 경합 — 전달은 liveness ①②와 조건 대기로 본다), 묶이지 않은 작업 영역 fixture의 `cancelledRuns` 순서.

## CI 실패 대응 (run 36313466550)

- 증상: `every_step_fixture_matches_on_in_memory_and_http_paths` — `orchestration-reassign-task-ok`의 두 경로가 갈림. 차이는 자식 **차단 보고 직후 `adoptManualChild` 단계**의 작업 영역: 한 경로는 coordinator 알림 `dispatching`·revision 7, 다른 경로는 `delivered`·revision 8.
- 원인: 보고가 만든 알림의 전달은 백그라운드(오늘과 같음)인데, 조건 대기(`until … delivered`)를 그 다음 GET에만 두어 비동기 완료 경계가 보고보다 한 단계 늦었다. 같은 구조가 `orchestration-agent-reassign-blocked-child-ok`에도 있었다.
- 수정: 생성기가 **모든 자식 보고(성공 기대) 바로 뒤**에 "그 보고의 알림이 `delivered`"를 기다리는 조건 대기 단계를 넣는다. 그 결과 보고 응답의 `notifications`도 결정적이 되어, 앞서 경로 비교에서 빼던 `notifications`(RACY)를 되돌려 **다시 비교한다**(뺀 값은 `cancelledRuns` 순서 하나만 남음).
- 재현·검증: CPU 부하(논리 코어 12개 모두 busy loop)에서 orchestration fixture만 8회 실행 — **이전 fixture 6/8 실패**(두 재배정 fixture), **새 fixture 8/8 통과**. 반복 통과만이 아니라 실패를 재현하는 조건에서 비교했다.

