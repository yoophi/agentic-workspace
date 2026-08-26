# Implementation Plan: Kiro CLI Session Resume

**Branch**: `036-kiro-session-resume` | **Date**: 2026-08-25 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/036-kiro-session-resume/spec.md`

## Summary

AW의 "기존 세션 재사용" 목록에 Kiro CLI 세션이 나타나고, 선택하면 이전 대화를 이어갈 수 있게 한다.

Kiro는 `~/.kiro/sessions/cli/`에 `<uuid>.json`(메타)과 `<uuid>.jsonl`(대화 로그) 쌍으로 세션을 남기고, ACP `loadSession: true`를 광고한다. AW에는 이미 provider별 세션 스캔·재개 경로가 있으나 `provider_kind_for()`가 `"kiro-cli"`를 모르기 때문에 목록이 비어 있다.

**접근**: 새 추상화를 만들지 않고 기존 `ProviderKind` + `scan_agent()` 경로에 Kiro 케이스를 추가한다. 실측 결과 Kiro의 `.json`/`.jsonl`이 정확히 짝을 이루므로 `.jsonl`만 순회하는 기존 스캐너를 그대로 재사용할 수 있고, 메타는 옆 `.json`에서 읽는다. 변경은 Rust 파일 2개로 한정되며 프론트엔드 변경은 없다.

Phase 0 조사에서 **spec의 전제 하나가 틀렸음을 발견해 수정했다** — `session_created_reason`으로 "사람이 시작한 세션"을 가려내려 했으나 실측 11건이 전부 `subagent`였다(사용자가 직접 친 대화 포함). 실제 노이즈는 대화 로그가 빈 세션이었고, 필터 기준을 그쪽으로 바꿨다. 상세는 [research.md](./research.md) R3·R4.

## Technical Context

**Language/Version**: Rust (edition 2024, workspace 기준) — 백엔드 전용 변경

**Primary Dependencies**: 기존 것만 사용 — `serde_json`(파싱), `walkdir`(스캔), `chrono`(타임스탬프), `anyhow`(오류)

**Storage**: 읽기 전용 파일 접근. Kiro가 소유한 `~/.kiro/sessions/cli/`를 읽기만 하며 쓰지 않는다.

**Testing**: `cargo test --lib` (fixture 기반 단위 테스트) + 앱에서의 수동 재개 검증([quickstart.md](./quickstart.md) §3.3)

**Target Platform**: macOS 데스크톱 (Tauri). 경로 규약은 `$HOME` 기반이라 Linux에서도 동일하게 동작한다.

**Project Type**: 데스크톱 앱의 Tauri 백엔드 (hexagonal)

**Performance Goals**: 세션 30건 이상에서 목록 2초 이내 (SC-003). 대화 로그는 최대 200줄만 읽어 대형 세션(실측 최대 5.3MB)에서도 상한을 유지한다.

**Constraints**:
- 목록 표시를 위해 대화 로그 전체를 읽지 않는다 (FR-010)
- 세션 하나의 파싱 실패가 목록 전체를 실패시키지 않는다 (FR-006)
- 세션 루트 밖을 읽지 않고 심볼릭 링크를 따라가지 않는다

**Scale/Scope**: 프로덕션 파일 2개 변경, 신규 함수 2개(`kiro_root`, `parse_kiro`), fixture 테스트 11종. 프론트엔드 0.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First** — **PASS**
  변경은 `apps/agentic-workbench/src-tauri/src/` 안으로 한정된다(`domain/provider_session.rs`, `infrastructure/fs_provider_session_repository.rs`). Kiro 세션 저장소 해석은 AW 전용 관심사이며 두 번째 소비자가 없으므로 `crates/*`로 올리지 않는다. 앱 간 import 없음.

- **Feature-Sliced Frontend Architecture** — **N/A**
  프론트엔드 변경 없음. 세션 목록 UI는 `ProviderSession`을 provider 무관하게 렌더링하므로 백엔드가 항목을 반환하면 그대로 표시된다.

- **Hexagonal Tauri Backend Architecture** — **PASS**
  `domain/provider_session.rs`에는 순수 타입(`ProviderKind` 배리언트, `provider_kind_for` 매핑)만 추가한다 — 파일시스템·Tauri 의존 없음. 파일 접근과 Kiro 형식 해석은 `infrastructure/fs_provider_session_repository.rs`의 어댑터에 둔다. 포트(`ports/provider_session_repository.rs`)와 Tauri 명령은 변경하지 않는다.

- **Shared Core Before Shared UI** — **N/A**
  공유 패키지/크레이트 신설 없음. 단일 앱 관심사다.

- **Atomic Cross-App Verification** — **N/A**
  `packages/*`·`crates/*` 변경이 없다. 검증은 AW 자체 테스트로 완결된다.

- **Documentation and Storybook** — **PASS**
  Storybook 대상 없음(신규 UI 없음). Kiro 세션 재개 지원 사실과 제약(`message_count` 근사, `.jsonl` 의존)을 `docs/*.md`에 반영한다. 대상 문서는 tasks 단계에서 확정한다.

- **Testing and Safety** — **PASS**
  파서(`parse_kiro`)는 헌장 요구대로 fixture 기반 테스트를 붙인다([contracts](./contracts/provider-session-repository.md) §3의 T-1~T-11). 파일 접근은 세션 루트로 한정하고 `WalkDir::follow_links(false)`를 유지한다. 세션 목록은 `SessionScope::Path`로 작업 디렉토리 범위를 강제해 다른 프로젝트 대화가 노출되지 않게 한다. 읽기 전용이므로 Kiro 데이터를 훼손하지 않는다.

**Post-Design 재평가 (Phase 1 이후)**: 설계 산출물([data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md))을 확정한 뒤 다시 확인했으며 위 판정은 그대로 유지된다. 새로운 위반이나 정당화가 필요한 복잡도는 없다 — Complexity Tracking 표는 비어 있다.

## Project Structure

### Documentation (this feature)

```text
specs/036-kiro-session-resume/
├── plan.md              # This file
├── spec.md              # Phase 0 이후 US3/FR-007/SC-005/Assumptions 수정됨
├── research.md          # Phase 0 output — 실측 조사 R1~R8
├── data-model.md        # Phase 1 output — 매핑·파생 규칙
├── quickstart.md        # Phase 1 output — 자동/수동 검증 절차
├── contracts/
│   └── provider-session-repository.md   # 포트 동작 계약 + 검증 시나리오
├── checklists/
│   └── requirements.md  # spec 품질 체크리스트
└── tasks.md             # Phase 2 output (/speckit-tasks — 아직 생성 안 됨)
```

### Source Code (repository root)

```text
apps/agentic-workbench/src-tauri/src/
├── domain/
│   └── provider_session.rs              # ProviderKind::Kiro, provider_kind_for 매핑 추가
├── infrastructure/
│   └── fs_provider_session_repository.rs # SessionRoots.kiro, kiro_root(), parse_kiro(), list() 분기
└── ports/
    └── provider_session_repository.rs   # 변경 없음 (시그니처 유지)

docs/
└── [tasks 단계에서 확정].md              # Kiro 세션 재개 지원 및 제약 기술
```

**Structure Decision**: 기존 hexagonal 배치를 그대로 따른다. 순수 타입은 `domain`, 파일 접근·형식 해석은 `infrastructure`. 신규 모듈이나 계층을 만들지 않는다 — Kiro는 기존 3개 provider와 같은 자리에 네 번째로 들어간다.

## Implementation Phases

구현은 아래 순서를 따른다. 각 단계는 독립적으로 검증 가능하다.

### Phase A — 도메인 매핑 (US1 기반)

`ProviderKind::Kiro` 배리언트와 `provider_kind_for("kiro-cli")` 매핑을 추가한다. 기존 매핑 테스트에 회귀 검증을 더한다(T-10, T-11).

이 단계만으로는 목록이 비어 있다 — `list()`가 아직 Kiro를 처리하지 않으므로 컴파일 오류(비망라 match)가 나는 것이 정상이며, Phase B에서 해소된다.

### Phase B — 스캔·파싱 어댑터 (US1, US2 완성)

`SessionRoots.kiro` 필드, `kiro_root()`, `parse_kiro()`를 추가하고 `list()`에 분기를 넣는다.

`parse_kiro`는 `.jsonl` 경로를 받아 옆의 `.json`에서 메타를 읽고, 로그에서 `message_count`를 센다([data-model.md](./data-model.md) §4). fixture 테스트 T-1~T-3, T-5~T-9를 여기서 붙인다.

이 단계가 끝나면 US1(목록 노출 + 재개)과 US2(식별 정보)가 동작한다.

### Phase C — 빈 세션 제외 (US3)

로그에 유효 이벤트가 없는 세션을 결과에서 제외한다(T-4). Phase B의 파서에 판별을 더하는 작은 변경이며, 별도 단계로 두는 이유는 US3가 P3이고 독립적으로 켜고 끌 수 있어야 하기 때문이다.

### Phase D — 실제 재개 검증 및 문서화

[quickstart.md](./quickstart.md) §3을 수행한다. 특히 §3.3(실제 `session/load` 재개)은 research R7에서 미검증으로 남긴 부분이라 **반드시 확인**해야 한다. 결과를 문서에 반영한다.

## Risks

| 리스크 | 영향 | 대응 |
|---|---|---|
| `session/load`가 Kiro에서 실제로 실패 | 기능의 핵심이 동작하지 않음 | Phase D에서 조기 검증. 실패 시 FR-009 경로(사용자 통보 + 새 세션 진행)가 최소 보장선이며, 원인 파악 후 별도 이슈로 분리 |
| `.json`만 있고 `.jsonl`이 없는 세션 발생 | 해당 세션 목록 누락 | 현재 데이터에 없음. 허용 가능한 열화로 판단하고 research R1에 기록 |
| Kiro 저장 형식 변경 | 파싱 실패 | 모든 필드를 optional로 다루고 모르는 `kind`를 무시한다. fixture 테스트가 회귀를 잡는다 |
| `message_count` 과소 계산 | 표시값 부정확 | 의도된 트레이드오프. data-model §4.1과 코드 주석에 명시 |

## Complexity Tracking

> Constitution Check에 위반이 없으므로 비어 있다.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (없음) | — | — |
