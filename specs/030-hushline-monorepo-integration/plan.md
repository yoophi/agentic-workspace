# Implementation Plan: Hushline 모노레포 편입 및 Agent Run 기능 추가

**Branch**: `030-hushline-monorepo-integration` | **Date**: 2026-07-21 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/030-hushline-monorepo-integration/spec.md`

## Summary

Hushline(YouTube 자막 생성 데스크톱 앱)을 `agentic-workspace` 모노레포의 `apps/hushline`로
편입하고, `agentic-workbench`의 ACP 기반 agent 통신 코어를 공유 crate `crates/acp-agent-core`
(+ TS 클라이언트 `packages/agent-client`)로 추출해 hushline이 소비하도록 한다. 이를 통해
hushline에서 (P2) 자막을 사용자가 원하는 방식으로 정리해 새 문서로 저장하고, (P3) 그 문서를
대상으로 세션 내 다회 대화를 수행한다. 접근 방식은 hexagonal 경계를 보존한 순수 코어 추출
(무동작 리팩터) → 모노레포 병합 → hushline 백엔드/프론트 얇은 접착층 배선 → walking skeleton
검증 → 기능 확장 순이다. 상세 설계 근거는 `docs/20260721-acp-agent-core-reuse-strategy.md`.

## Technical Context

**Language/Version**: Rust (workbench edition 2024 / hushline edition 2021 — 공유 crate는
edition 2021로 핀하여 두 앱 모두 소비 가능), TypeScript ~5.6, React 18.3(hushline)·19(workbench)

**Primary Dependencies**: Tauri 2, `agent-client-protocol` 0.15.0, tokio(rt-multi-thread/process/
sync/time), `@tauri-apps/api` 2. (axum 기반 MCP 서빙은 본 기능 범위 밖 — Phase 4 후속)

**Storage**: 앱 관리 출력 폴더의 구조화 JSON 파일. 기존 Transcript 결과 저장 방식을 따르는
OrganizedDocument·ChatSession 영속. ACP 세션 재개(resume) 영속은 범위 밖(noop store 사용).

**Testing**: `cargo test`(Rust 코어/앱), `pnpm check-types`(TS), `crates/acp-agent-core`의
domain/ports/application 순수 로직 단위·픽스처 테스트. hushline `test` 스크립트는 현재
`cargo test` 기반.

**Target Platform**: macOS/Windows/Linux 데스크톱(Tauri 2 webview).

**Project Type**: desktop-app (Tauri 2 백엔드 + React FSD 프론트, pnpm/Turbo + Cargo workspace).

**Performance Goals**: agent 스트리밍 출력이 즉시 진행 표시로 반영되어 "반응 없음"으로 느껴지지
않을 것(SC-003). 동시 run 안전 상한 적용(FR-011).

**Constraints**: hexagonal 경계(도메인 Tauri/FS 비의존), FSD 레이어 준수, 앱 관리 경로 밖
파일 접근 금지(경로·크기·UTF-8 검증), run/세션 소유 범위 검증, 앱 간 직접 import 금지.

**Scale/Scope**: 단일 사용자 데스크톱, 소수의 동시 run. 편입 앱 1개 + 신규 공유 crate 1개
(+ 선택적 TS 패키지 1개).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First** — **PASS**. hushline 앱 코드는 `apps/hushline/*`, 재사용 Rust
  코어는 신규 `crates/acp-agent-core`, 재사용 TS는 `packages/agent-client`에 둔다. hushline은
  `agentic-workbench`에서 직접 import하지 않고 workspace crate/package만 소비한다. 공유 crate는
  두 앱(workbench·hushline)이 실제 소비하므로 "2개 이상 소비" 요건 충족(원칙 I). 근거:
  `docs/20260721-acp-agent-core-reuse-strategy.md` 공유 경계 표.
- **Feature-Sliced Frontend Architecture** — **PASS**. 정리/대화 액션은 `apps/hushline/src/
  features/*`, 자막·정리문서 모델·어댑터는 `entities/*`, agent 호출 래퍼 소비는 `shared/api`,
  화면은 `pages/*`·`widgets/*`. shadcn 컴포넌트는 `components/ui` 유지.
- **Hexagonal Tauri Backend Architecture** — **PASS**. hushline `src-tauri`는 기존
  domain/application/ports/adapters 유지. agent command(inbound)는 입력 검증·위임만 하고,
  공유 코어의 application/ports를 통해 실행. 이벤트 sink·저장은 infrastructure. 공유 crate 내부도
  동일 경계(domain Tauri 비의존)로 추출.
- **Shared Core Before Shared UI** — **PASS**. agent run domain·ports·application·ACP 어댑터
  (순수 코어)만 공유. UI는 공유하지 않고 hushline 고유로 둔다(요구 미수렴, 원칙 IV).
- **Atomic Cross-App Verification** — **PASS(계획)**. `crates/acp-agent-core` 추출은 소비 앱
  `agentic-workbench`의 Rust 테스트/체크와 신규 소비 앱 `hushline`의 Rust 체크를 모두 통과해야
  완료. `packages/agent-client` 추출 시 소비 앱 `check-types` 통과 필수(원칙 V).
- **Documentation and Storybook** — **PASS**. 설계 문서 `docs/20260721-acp-agent-core-reuse-
  strategy.md` 유지·갱신. 신규 재사용 UI가 생기면 Storybook atomic 분류 등록(현재는 앱 고유 UI라
  최소).
- **Testing and Safety** — **PASS(계획)**. 공유 코어 domain/ports/application 순수 로직에
  단위·픽스처 테스트. 자막/정리문서 파일 접근은 경로·크기·UTF-8 검증, agent run/세션은 소유
  범위(세션/창) 검증, 부작용 도구엔 권한 정책(FR-009·FR-010·FR-012).

**결과: 모든 게이트 PASS — Complexity Tracking 불필요.**

## Project Structure

### Documentation (this feature)

```text
specs/030-hushline-monorepo-integration/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Tauri command + RunEvent + TS client 계약)
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
# 신규 공유 Rust 코어 (workbench에서 추출):
crates/acp-agent-core/src/
├── domain/              # run, events, agent, permission, acp_session (Tauri 비의존)
├── ports/               # session_launcher, session_handle, session_registry, event_sink, ...
├── application/         # start_agent_run, send_prompt, cancel_agent_run, set_permission_mode, ...
└── infrastructure/      # acp/* (runner/client/transport), agent_session_registry, permission_broker

# 신규 공유 TS 클라이언트 (workbench에서 추출):
packages/agent-client/src/
├── types.ts             # AgentRunRequest, RunEventEnvelope, PermissionMode (Rust와 1:1)
└── repository.ts        # startAgentRun, sendPromptToRun, cancelAgentRun, listenRunEvents

# 편입 앱 (youtube-whisper-stt에서 이동):
apps/hushline/src/
├── app/                 # 기존
├── pages/ widgets/      # 기존 + 정리/대화 UI
├── features/            # transcribe-video(기존) + organize-transcript, chat-with-document(신규)
├── entities/            # transcription(기존) + organized-document(신규)
└── shared/api/          # tauri.ts(기존) + @yoophi/agent-client 재노출

apps/hushline/src-tauri/src/
├── domain/              # 기존 + OrganizedDocument, ChatSession
├── application/         # 기존 + 정리/대화 유스케이스(공유 코어 소비)
├── adapters/ (inbound)  # tauri.rs: agent command 등록 + HushlineAgentSink
└── ports.rs             # 기존

# 기존 소비 앱 (추출 후 새 crate 의존으로 교체):
apps/agentic-workbench/src-tauri/  # acp-agent-core 소비
apps/agentic-workbench/src/        # @yoophi/agent-client 소비

# 문서:
docs/20260721-acp-agent-core-reuse-strategy.md
```

**Structure Decision**: 데스크톱 Tauri 앱 편입 + Rust 코어/TS 클라이언트 공유. 재사용은
`crates/acp-agent-core`(Rust 순수 코어)와 `packages/agent-client`(TS 계약)로 한정하고, 이벤트
sink·command 조립·저장 경로·MCP tool 정의 등 Tauri/앱 결합 부분은 각 앱 `src-tauri`에 남긴다.
공유 crate는 `agentic-workbench`와 `hushline` 두 앱이 소비한다.

## Complexity Tracking

> Constitution Check 전 항목 PASS이므로 해당 없음.
