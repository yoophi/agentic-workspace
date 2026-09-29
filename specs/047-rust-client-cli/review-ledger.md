# 047 설계/구현 리뷰 기록

현재 specify/plan draft, 구현·tests 없음. base20fcd5f의별도047 branch, 0456e4bf30·0469b1b2e2 보존. source dependency(protocol/lifecycle/TSclient) diff0 actual 확인. 046 partial7/31 및045macOS T010/T016 PENDING 유지. 새 Linux/Windows 구현/검증0. 관련 없는 untracked docs/code-review-app-migration.md 보존/제외.

주요 리뷰 질문: client-only dependency 경계、full fault/outcome/replayed 보존、unknown operation/key/epoch generation과 double send、descriptor/identity credential 순서、agent authority fallback、applied cursor/live-first recovery、quota/cancel/stdout、generic mutation productiongate。approved 순서는 OCR host manual 전체Markdown → Codex --wait이며 둘의scope/verdict/findings반영을 별도 기록한다. 그전 tasks/implementation 미실행.
