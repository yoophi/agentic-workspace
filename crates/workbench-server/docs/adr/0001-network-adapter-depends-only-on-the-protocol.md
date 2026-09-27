---
status: accepted
date: 2026-09-27
---

# 네트워크 어댑터는 protocol에만 기대는 별도 크레이트다

3단계 HTTP/WS 어댑터를 `crates/workbench-server`에 둔다. 이 크레이트는 `workbench-protocol`의 `Workbench` trait만 부르고 `workbench-core`를 모른다. 자격 증명 해석(`CredentialResolver`)과 서버 정보(`ServerInfo`)는 포트로 받는다. 데스크톱 앱(3단계)과 독립 서버(5단계)가 같은 router를 조립하고, core의 계약·이벤트 suite는 테스트 전용 harness 대신 이 운영 router로 두 경로 parity를 잰다.

## Considered Options

- core 안의 `inbound/http` 모듈 — core가 axum·tower 운영 의존을 갖고, 5단계 서버가 core 전체를 끌어온다. 인증 정책이 도메인 코드 옆에 섞인다.
- AW 안의 어댑터 — 5단계 서버가 Tauri 앱 크레이트에 기대게 된다.
- 별도 크레이트, protocol만 의존(채택) — 조립하는 쪽이 인증·서버 정보를 주입한다. core는 dev-dependency로만 이 크레이트를 쓴다.

## Consequences

- 데스크톱 토큰·MCP 토큰 같은 자격 증명 종류는 조립 쪽(AW `infrastructure/workbench_http.rs`)이 정한다. 크레이트는 `DesktopTokenIssuer`·`ChainResolver`·`StaticResolver` 부품만 준다.
- 출처 정책(`OriginPolicy`)은 이 크레이트가 정본이고 AW MCP 서버도 같은 것을 쓴다(접두사 비교 결함 수정).
- 검증: `workbench-core/tests/http_*.rs`, `contract_suite.rs`·`event_contract_suite.rs`(운영 router).
