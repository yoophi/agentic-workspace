# 047 구현 검증 기록

## 2026-09-29: T001–T004

환경: Darwin 24.6.0 arm64, macOS 15.6.1. macOS14+/설치본 검증 아님. 설계 기준 bd6099c, 이후 tasks 생성과 client/CLI 공통 기반 작업.

| 명령/검증 | 실제 결과 | 범위 |
|---|---|---|
| `cargo check -p workbench-client -p aw-cli` | exit0 | 신규 crate와 CLI scaffold 빌드 |
| `cargo metadata --no-deps --format-version 1` | exit0 | 두 package workspace membership 확인 |
| `cargo tree -p workbench-client -e normal` 및 aw-cli | exit0 | production graph에 host/core/server/Tauri 없음 |
| 최초 `cargo test -p workbench-client --test limits` | exit101, unresolved limits module | 구현 전 실패 확인 |
| 최초 `cargo test -p workbench-client --test attempt` | exit101, unresolved attempt module | 구현 전 실패 확인 |
| `cargo test -p workbench-client -p aw-cli` | exit0, limits7 + attempt11 = 18개 통과, 실패/무시/filtered0 | pure limits/attempt; CLI 자체 unit test0 |
| `cargo clippy -p workbench-client -p aw-cli --all-targets -- -D warnings` | exit0 | 새 client/CLI 모든 현재 target |
| `cargo fmt -p workbench-client -p aw-cli` | exit0 | 현재 신규 Rust 파일 |

Cargo.lock 기존 package version 제거/교체0. 신규 workspace package2와 hmac0.12.1/subtle2.6.1 추가 및 digest subtle edge만 변경했다. 기존 hyper1.10.1/hyper-util0.1.20/http-body-util0.1.3/tokio-tungstenite0.24.0/sha2 0.10.9 유지. hmac0.12는 기존 digest0.10/sha2 0.10 계열과 연결된다.

순수 모델의 request 입력은 불변이며 epoch/instance 교체와 generation 불일치 완료를 거절한다. full reply/revision/replayed와 fault details/outcome을 보존하고 Debug는 private input/reply/fault를 숨긴다. Accepted는 최종 Applied로 판정하지 않는다. 실제 network/store/CLI command wiring은 아직 없다. CLI 현재 entry는 unavailable/exit8 scaffold이며 구현 완료 CLI로 배포할 수 없다.

기존 protocol 및 consumer source 변경0. shared crate의 현재 consumer인 aw-cli를 함께 check/test/clippy했다. 향후 HTTP/WS/identity 연결 시 protocol/host/server/AW 및 TS parity 검증은 T040에 남는다. 원 전체8gate, 실제 merged044 server 통합, 구현 OCR/Codex 리뷰, PR/merge 및 045/046 readiness는 미완료다.
