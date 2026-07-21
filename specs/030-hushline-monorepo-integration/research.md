# Phase 0 Research: Hushline 모노레포 편입 및 Agent Run

본 문서는 plan.md의 Technical Context에서 결정이 필요한 항목을 해소한다.
상위 설계 근거: `docs/20260721-acp-agent-core-reuse-strategy.md`.

## R1. 코어 공유 방식 (병합 vs 배포)

- **Decision**: 모노레포 병합(방식 A). hushline을 `agentic-workspace`로 편입하고,
  공유 코어를 workspace crate/package(path 의존)로 소비.
- **Rationale**: 코어가 막 추출돼 자주 바뀌는 초기 + Rust·TS 이중 계약(camelCase 직렬화)
  드리프트 위험 + 동일 스택 솔로 개발. 한 CI에서 `cargo test`+`check-types`로 원자적 검증.
  git-core 선례(`docs/git-feature-sharing-monorepo-strategy.md`)와 일치.
- **Alternatives considered**: 패키지 배포(C) — 코어 안정 + 외부 소비자 발생 시 승격 대상.
  현재는 왕복 비용·이중 채널 버전 정합 부담이 커 기각. git subtree(B) — 임시방편, 기각.

## R2. 추출 범위 (무엇을 crate로 뽑는가)

- **Decision**: 최소 통신 코어만. domain{run, events, agent, permission, acp_session,
  agent_tool_candidate} + ports 전체 + application{start_agent_run, send_prompt,
  cancel_agent_run, set_permission_mode, cancel_prompt_and_send, steer_prompt,
  agent_run_errors} + infrastructure{acp/*, agent_session_registry, permission_broker,
  agent_catalog}.
- **Rationale**: 확인 결과 `provider_session`·`agent_run_settings`(복잡 override)·goal·
  saved_prompt·git·mcp·창 메뉴는 통신 유스케이스에 물려 있지 않아 분리 가능. hushline은
  "run 실행 + 이벤트 스트림 + 다회 프롬프트"만 필요.
- **Alternatives considered**: 주변 기능까지 통째 추출 — 불필요 결합·이식 비용 증가로 기각.

## R3. Rust edition / 버전 divergence

- **Decision**: `crates/acp-agent-core`를 edition 2021로 핀. workbench(2024)·hushline(2021)
  모두 소비 가능(Cargo는 크레이트별 edition 혼용 허용).
- **Rationale**: 소비 앱 edition과 무관하게 crate가 컴파일되면 됨. hushline을 2024로 올리는
  변경은 편입 회귀 위험을 키우므로 이번 범위에서 하지 않음.
- **Alternatives considered**: 전 앱 edition 2024 통일 — 별도 작업으로 분리(범위 밖).

## R4. TS 계약 드리프트 방지

- **Decision**: 1차는 `packages/agent-client`에 수기 TS 타입 유지 + Rust↔TS 계약 픽스처
  테스트(직렬화 스냅샷)로 고정. 자동 생성(ts-rs/specta) 도입은 후속 개선으로 표기.
- **Rationale**: 병합으로 한 CI에서 즉시 검증되므로 수기 + 스냅샷으로 충분. 자동 생성은
  빌드 파이프라인 비용이 있어 코어 안정 후 도입이 합리적.
- **Alternatives considered**: 즉시 ts-rs 도입 — 초기 오버헤드 대비 이득 낮아 후속.

## R5. 자막 컨텍스트 전달 방식

- **Decision**: 정리(P2)·대화(P3) 모두 run의 `cwd`를 자막 결과 폴더로 두고, goal/프롬프트에서
  자막 파일을 참조("이 자막을 ~방식으로 정리: `transcript.txt`"). 에이전트가 파일 도구로 읽음.
- **Rationale**: 대용량 자막을 프롬프트에 인라인하면 컨텍스트 낭비·한계. 파일 참조가 확장적.
  ACP 세션은 첫 로드 후 후속 프롬프트에서 맥락 유지.
- **Alternatives considered**: 자막 전문을 프롬프트에 인라인 — 짧은 자막엔 가능하나 기본값으론 기각.

## R6. 정리 결과 저장 주체

- **Decision**: 에이전트의 `AgentMessage` 스트림을 hushline이 수집해 OrganizedDocument로 저장(방식 b).
  에이전트 직접 파일 write는 옵션(기본 비활성).
- **Rationale**: 저장 위치·포맷·파일명·기존 결과 카드 UI와의 일관성을 hushline이 통제. 권한 흐름
  마찰 최소화.
- **Alternatives considered**: 에이전트가 직접 파일 생성(a) — 권한 흐름 필요·저장 위치 통제 약화로 옵션화.

## R7. 세션 지속성(resume)

- **Decision**: 1차는 세션 내 대화만. ACP 세션 재개 영속은 `noop_acp_session_store` 사용
  → 코어의 Tauri 경로 결합 지점을 이번엔 건드리지 않음. OrganizedDocument·ChatSession 로그
  자체는 hushline이 JSON으로 영구 저장.
- **Rationale**: 앱 재시작 후 대화 이어가기는 부가 가치이며 저장/재개 설계 비용이 큼. MVP는
  세션 내 유지로 충분(FR-008).
- **Alternatives considered**: 즉시 세션 재개 지원 — `AcpSessionStore` 포트 주입 + 경로 결정
  필요. 후속 스펙으로 분리.

## R8. 권한 / 부작용 도구 정책

- **Decision**: 정리는 읽기 위주 흐름(방식 b)이라 마찰 적음. 대화 중 에이전트 도구 사용은
  `ReadOnly`/`Plan` 모드로 시작하고, 파일 쓰기 등 부작용은 코어 Permission 흐름을 통해 사용자
  승인 또는 안전 기본 제한 적용. 런타임 승격은 `set_permission_mode`.
- **Rationale**: 에이전트는 외부 프로세스이므로 부작용은 명시 승인 경유가 안전(FR-010).
- **Alternatives considered**: 전면 auto-allow — 안전성 위배로 기각.

## R9. 동시 실행 상한 / 소유 범위

- **Decision**: 코어 registry의 동시 run 상한을 그대로 사용하되 env 이름을 중립(`ACP_MAX_RUNS`)으로
  리네임(하위호환 유지). run 소유자는 generic string(세션/창 식별자)로 두고, 창 종료 시 소유 run
  정리 로직은 hushline 접착층에서 연결.
- **Rationale**: registry는 Tauri 비의존 generic owner 구조라 재사용 가능(FR-009·FR-011).
- **Alternatives considered**: 상한/소유 로직 재작성 — 불필요, 기각.

## R10. 편입 절차(히스토리 보존)

- **Decision**: hushline을 `apps/hushline`로 히스토리 보존하며 import하고, 루트
  `pnpm-workspace.yaml`·`Cargo.toml` workspace members·`turbo.json`에 등록. 앱 패키지명은
  기존 `@yoophi/hushline` 유지.
- **Rationale**: 동일 workspace 레이아웃이라 편입 비용 낮음. 히스토리 보존으로 추적성 유지.
- **Alternatives considered**: 히스토리 버리고 스냅샷 복사 — 추적성 손실로 지양.

## 미해결 사항

- 없음. plan.md의 Technical Context에 NEEDS CLARIFICATION 없음.
