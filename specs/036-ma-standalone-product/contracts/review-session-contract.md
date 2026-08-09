# Contract: Review Session Service

## Backend boundary

Tauri command는 요청을 검증 가능한 DTO로 변환한 뒤 application service에 위임한다. repository, clock, fingerprint와 native shell은 port이며 filesystem/JSON 구현은 infrastructure에만 둔다.

## Commands

| 명령 | 입력 | 성공 결과 | 주요 오류 |
|---|---|---|---|
| `load_review_session` | root ID, relative path, current fingerprint | session 또는 신규 draft | path/document/storage 오류 |
| `save_review_session` | full session, expected revision | 증가한 revision의 session | `revision_conflict`, validation, storage |
| `reconcile_review_session` | session ID, parsed document evidence | attachment 결과와 session | unsupported schema/document |
| `propose_document_relink` | missing session ID, root scan snapshot | 0 또는 1 proposal | ambiguous는 proposal 없음 |
| `confirm_document_relink` | proposal token, expected revision | 변경된 identity/session | expired proposal/conflict |
| `trash_review_session` | session ID, expected revision | trash receipt | conflict/not found |
| `restore_review_session` | trash receipt | restored session | expired/conflicting target |
| `export_feedback` | session ID, annotation IDs, resolved opt-in, format | text payload | invalid selection/storage |

## Save invariants

- session 전체가 atomic 저장 단위다.
- `expectedRevision`이 current와 다르면 disk를 덮어쓰지 않는다.
- 저장 성공 전에 frontend에 성공을 알리지 않는다.
- 실패한 temp 파일은 current를 대체하지 않는다.
- 저장 성공 시 snapshot 5개 retention을 적용하되 active current는 정리하지 않는다.
- unknown future schema는 migration 없이 보존하고 recoverable error를 반환한다.

## Validation invariants

- annotation ID와 group ID는 session 안에서 유일해야 한다.
- anchor offsets는 동일 fingerprint에서만 위치 증거로 사용한다.
- resolved annotation을 export하려면 caller가 opt-in해야 한다.
- 선택 annotation은 모두 해당 session에 속해야 한다.
- open change-request/delete가 있는 approval은 확인 token이 필요하다.
- approved session에 annotation 생성 시 decision 전환 확인이 필요하다.

## Reconciliation result

각 annotation은 `attached`, `conflict`, `orphan`, `missing` 중 정확히 하나를 가진다. service는 자동 결과의 근거(`block-id-exact` 또는 `text-context-unique`)를 함께 반환한다. fuzzy match나 복수 후보 중 임의 선택은 금지한다.

rename/move proposal은 같은 root, 동일 fingerprint, 단 하나의 새 relative path일 때만 발급한다. proposal은 scan revision과 만료 시각에 묶고 사용자 confirm 없이는 저장 identity를 변경하지 않는다.

## Recovery contract

current JSON이 손상되면 원본을 `corrupt`로 이동하고 newest valid snapshot부터 검증한다. 복구 성공을 UI에 알리고 새 revision으로 저장한다. snapshot도 없으면 데이터 없음으로 가장하지 않고 recovery action이 가능한 오류를 제공한다.
