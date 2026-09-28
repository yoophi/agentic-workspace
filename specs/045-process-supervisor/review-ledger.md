# 045 설계 리뷰 반영 ledger

## 고정 리뷰 범위

- base: `20fcd5fdcf633ae06792d51a9b963e3857909440`
- reviewed head: `87c2fce8a5ccabbbcc179fb3626dc4ad77efe79d`
- 변경 파일: 11개 (`.specify/feature.json` 1개, Markdown 10개)
- OCR 자동 분류: total 11, reviewable 1, Markdown 10개는 `unsupported_ext` excluded
- OCR host 설계 검토: 11/11 실제 읽음, skipped 0, `needs-attention` (High 4, Medium 1)
- Codex adversarial: `review-mul90gfv-g9i2tc`, 11/11, `needs-attention` (Critical 1, High 4, Medium 2)

OCR 자동 분류 coverage와 host 수동 설계 coverage를 합쳐서 100%라고 표현하지 않는다. 위 두 수치를 별도로 유지한다.

## finding 판정과 반영

| ID | 출처 | 판정 | 중복 | 반영 |
|---|---|---|---|---|
| D1 | OCR High, Codex Critical | 유효 | 동일 근본 원인 | `Adopted → Published/Active/Aborting` 단일 CAS, loser typed result, ambiguous storage quarantine, 양쪽 winner fixture |
| D2 | OCR High, Codex High | 유효 | 동일 | Published/domain result/attempt-keyed outbox atomic transaction, reconnect replay와 projection dedupe |
| D3 | OCR High, Codex High | 유효 | 동일 | read-only helper의 transient domain 의미와 모든 ServerOwned child의 durable containment recovery anchor 분리 |
| D4 | OCR High, Codex High | 유효 | 동일 | live server의 keeper-exit 감시·즉시 cleanup 인계, combined crash startup reconcile, adoption/publication/cleanup 각 death fixture |
| D5 | OCR Medium, Codex High | 유효 | 동일 | checklist readiness PASS 해제, target spike 전 consumer migration 금지 |
| D6 | Codex Medium | 유효 | 없음 | GE/HL/MA production spawn source를 정확한 path로 inventory에 열거, app-wide wildcard 금지 |
| D7 | Codex Medium | 유효 | 없음 | protocol incomplete-frame deadline/minimum progress, slow-loris와 endless-valid-frame fixture |

## 남은 prerequisite 증거

- macOS/Linux: env-clear+exec descendant, actual process inventory permission, 재사용 안전 handle과 signal 원자성, keeper-only/server+keeper hard kill을 격리 spike로 검증한다.
- Windows: suspended create, Job assign/resume, breakaway denial, server crash kill-on-close를 실제 target에서 검증한다.
- 어느 target도 문서상 설계나 nonce 상속 fixture만으로 PASS 처리하지 않는다.
- prerequisite가 실패하면 전체 목표를 즉시 포기하거나 scope를 줄이지 않는다. 실패 target의 API/권한 근거와 대안을 새 설계 리뷰에 올리고 production consumer migration은 보류한다.
