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
