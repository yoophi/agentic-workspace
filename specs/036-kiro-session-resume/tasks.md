---

description: "Task list for Kiro CLI session resume"
---

# Tasks: Kiro CLI Session Resume

**Input**: Design documents from `/specs/036-kiro-session-resume/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/provider-session-repository.md)

**Tests**: 사용자 여정 테스트는 spec이 요구하지 않았으므로 생략한다. 다만 **헌장이 요구하는 테스트는 필수**다 — `parse_kiro`는 파서이므로 fixture 기반 단위 테스트를 반드시 붙인다. 계약 시나리오 T-1~T-11은 [contracts §3](./contracts/provider-session-repository.md#3-검증-시나리오)에 정의되어 있다.

**Organization**: 사용자 스토리별로 묶어 각 스토리를 독립적으로 구현·검증할 수 있게 한다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능 (다른 파일, 미완료 의존 없음)
- **[Story]**: 해당 사용자 스토리 (US1, US2, US3)
- 모든 태스크에 정확한 파일 경로를 포함한다

## Path Conventions

- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,infrastructure,ports}`
- **Documentation**: `docs/[english-file-name].md`

이 기능은 백엔드 전용이다. 프론트엔드 변경 없음 — 세션 목록 UI는 `ProviderSession`을 provider 무관하게 렌더링한다.

---

## Phase 1: Setup (기준선 확보)

**Purpose**: 회귀 판정의 기준선을 만든다. 이 기능은 기존 코드에 provider 하나를 추가하는 작업이라 신규 프로젝트 셋업이 없다.

- [X] T001 `cd apps/agentic-workbench/src-tauri && cargo test --lib`를 실행해 현재 통과 테스트 수를 기록한다 (변경 전 기준선, 이후 회귀 판정에 사용)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 사용자 스토리가 의존하는 도메인 매핑과 어댑터 골격. 이 단계가 끝나야 어떤 스토리든 시작할 수 있다.

**⚠️ CRITICAL**: T002~T005는 서로 맞물려 있어 함께 완료해야 컴파일이 통과한다. `ProviderKind`에 배리언트를 추가하면 `list()`의 match가 비망라 상태가 되기 때문이다.

- [X] T002 `apps/agentic-workbench/src-tauri/src/domain/provider_session.rs`의 `ProviderKind` enum에 `Kiro` 배리언트를 추가한다
- [X] T003 `apps/agentic-workbench/src-tauri/src/domain/provider_session.rs`의 `provider_kind_for()`에 `"kiro-cli" => Some(ProviderKind::Kiro)` 매핑을 추가하고, 지원하지 않는 provider가 `None`을 반환하는 기존 동작을 유지한다
- [X] T004 `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs`의 `SessionRoots` 구조체에 `kiro: Option<PathBuf>` 필드를 추가한다
- [X] T005 `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs`에 `kiro_root()` 메서드를 추가한다 — `KIRO_SESSION_DIR` 환경변수를 우선하고, 없으면 `$HOME/.kiro/sessions/cli`를 반환한다 (기존 `claude_root`/`codex_root`/`pi_root`와 동일한 패턴, [contracts C-2](./contracts/provider-session-repository.md))
- [X] T006 `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs`의 `list()` match에 `ProviderKind::Kiro => scan_agent(agent_id, self.kiro_root()?, scope, parse_kiro)` 분기를 추가한다 (`parse_kiro`는 T008에서 구현하며, 그 전까지는 컴파일을 위한 최소 스텁을 둔다)
- [X] T007 [P] `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs`의 테스트 모듈에 `repo_with_kiro(root: PathBuf)` 헬퍼를 추가한다 (기존 `repo_with_codex` 패턴을 따른다)

**Checkpoint**: `cargo check`가 통과하고, `provider_kind_for("kiro-cli")`가 `Some(ProviderKind::Kiro)`를 반환한다.

---

## Phase 3: User Story 1 - 워크트리에서 이전 Kiro 대화 이어가기 (Priority: P1) 🎯 MVP

**Goal**: Kiro 세션이 목록에 나타나고, 선택하면 이전 맥락을 유지한 채 재개된다.

**Independent Test**: Kiro CLI로 임의 디렉토리에서 대화를 만든 뒤, AW에서 그 디렉토리를 열고 Kiro CLI + 기존 세션 재사용으로 해당 대화가 목록에 뜨고 재개되는지 확인한다 ([quickstart §3.2, §3.3](./quickstart.md)).

### 구현

- [X] T008 [US1] `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs`에 `parse_kiro(agent_id: &str, path: &Path) -> Result<Option<ProviderSession>>`를 구현한다. `path`는 `<uuid>.jsonl`이며, 같은 stem의 `<uuid>.json`에서 메타를 읽는다. 메타 파일이 없거나 JSON 파싱에 실패하면 `Ok(None)`을 반환해 그 세션만 건너뛴다 ([data-model §3.1](./data-model.md))
- [X] T009 [US1] T008의 `parse_kiro`에서 메타 필드를 `ProviderSession`으로 매핑한다 — `session_id`→`id`(없으면 `file_stem_id(path)`), `cwd`→`cwd`, `created_at`→`created_at`, `updated_at`→`updated_at`, `session_state.rts_model_state.model_info.model_id`→`model`. `branch`와 `source`는 `None`으로 둔다 ([data-model §1, §5](./data-model.md))
- [X] T010 [US1] T008의 `parse_kiro`에서 `message_count`를 산출한다 — `read_json_lines(path, 200)`으로 로그를 읽고 `kind`가 `Prompt` 또는 `AssistantMessage`인 이벤트만 센다. `ToolResults`/`Compaction`과 모르는 `kind`는 세지 않는다. 200줄 상한으로 대형 세션에서 과소 계산된다는 점을 코드 주석에 남긴다 ([data-model §4.1](./data-model.md))

### 테스트 (헌장 필수 — 파서)

- [X] T011 [P] [US1] `fs_provider_session_repository.rs` 테스트 모듈에 T-1을 추가한다 — 정상 세션 1건(메타+로그) fixture를 만들고 모든 필드가 매핑대로 채워지는지 검증한다
- [X] T012 [P] [US1] `fs_provider_session_repository.rs` 테스트 모듈에 T-2/T-3을 추가한다 — `SessionScope::Path`가 세션 `cwd`와 일치할 때 1건, 불일치할 때 0건을 검증한다
- [X] T013 [P] [US1] `fs_provider_session_repository.rs` 테스트 모듈에 T-8을 추가한다 — Kiro 루트 디렉토리가 없을 때 `Ok(vec![])`를 반환하는지 검증한다 ([contracts C-3](./contracts/provider-session-repository.md))
- [X] T014 [P] [US1] `fs_provider_session_repository.rs` 테스트 모듈에 T-9를 추가한다 — `Prompt`+`AssistantMessage`만 계수하고 `ToolResults`는 제외하는지 검증한다
- [X] T015 [P] [US1] `apps/agentic-workbench/src-tauri/src/domain/provider_session.rs`의 기존 `provider_kind_mapping` 테스트에 T-10/T-11을 반영한다 — `"kiro-cli"`가 `Kiro`로 매핑되고 기존 3종 매핑이 불변인지 검증한다

### 실제 동작 검증

- [X] T016 [US1] [quickstart §3.2](./quickstart.md)를 수행해 실제 Kiro 세션이 목록에 나타나는지 확인한다 (`jq -r .cwd ~/.kiro/sessions/cli/*.json`로 대상 디렉토리를 먼저 확인)
- [X] T017 [US1] ⚠️ [quickstart §3.3](./quickstart.md)을 수행해 **실제 세션 재개**를 검증한다. research R7에서 `session/load` 실동작을 확인하지 못했으므로 이 태스크가 기능 성립의 관문이다. 실패하면 원인을 기록하고 FR-009(실패 통보 + 새 세션 진행 가능)가 지켜지는지 확인한 뒤 별도 이슈로 분리한다

**Checkpoint**: US1이 독립적으로 동작한다 — Kiro 세션 목록이 뜨고 재개된다. 이 시점이 MVP다.

---

## Phase 4: User Story 2 - 여러 세션 중 올바른 것 식별하기 (Priority: P2)

**Goal**: 목록에서 제목과 시각으로 세션을 구분할 수 있다.

**Independent Test**: 같은 디렉토리에서 서로 다른 주제의 Kiro 대화를 2건 이상 만든 뒤, 목록에서 제목과 시각으로 구분되는지 확인한다.

**Note**: 최근 활동 우선 정렬(US2 시나리오 2)은 `application/list_provider_sessions.rs`가 `updated_at`을 RFC3339로 파싱해 이미 처리한다. T009에서 `updated_at`을 채우면 별도 작업 없이 충족되며, T019에서 확인만 한다.

- [X] T018 [US2] `fs_provider_session_repository.rs`의 `parse_kiro`에서 `title`을 매핑한다 — 메타의 `title`을 우선 사용하고, 없으면 로그의 첫 `Prompt` 이벤트 `data.content`에서 발췌한다. 기존 `extract_codex_user_text` 패턴을 참고하되 Kiro의 이벤트 구조에 맞춘다 ([data-model §4.3](./data-model.md))
- [X] T019 [P] [US2] `fs_provider_session_repository.rs` 테스트 모듈에 T-5를 추가한다 — 메타에 `title`이 없고 로그에 `Prompt`가 있을 때 첫 프롬프트에서 제목이 발췌되는지 검증한다
- [X] T020 [US2] [quickstart §3.2](./quickstart.md)에서 목록 항목에 제목·시각이 보이고 최근 세션이 위에 오는지 확인한다 (정렬은 기존 use case가 처리하므로 코드 변경 없이 동작해야 한다)

**Checkpoint**: US1 + US2가 함께 동작한다 — 여러 세션을 구분해 고를 수 있다.

---

## Phase 5: User Story 3 - 대화가 없는 빈 세션 걸러내기 (Priority: P3)

**Goal**: 이어갈 내용이 없는 빈 세션이 목록에 뜨지 않는다.

**Independent Test**: 대화 없이 연결만 맺은 세션을 만든 뒤([quickstart §3.4](./quickstart.md)) 목록에 나타나지 않는지 확인한다.

**Note**: 판별은 **로그 기준**이며 제목 유무로 하지 않는다. 대화 중이라 제목이 아직 없는 세션을 잘못 제외하지 않기 위해서다 ([research R4](./research.md)).

- [X] T021 [US3] `fs_provider_session_repository.rs`의 `parse_kiro`에서 로그에 유효 이벤트가 하나도 없으면 `Ok(None)`을 반환해 목록에서 제외한다 ([contracts C-7](./contracts/provider-session-repository.md))
- [X] T022 [P] [US3] `fs_provider_session_repository.rs` 테스트 모듈에 T-4를 추가한다 — 로그가 0바이트인 세션이 결과에서 제외되는지, 그리고 제목은 없지만 로그에 내용이 있는 세션은 제외되지 **않는지** 함께 검증한다 (US3 시나리오 2)
- [X] T023 [US3] [quickstart §3.4](./quickstart.md)를 수행해 실제로 빈 세션을 만든 뒤 목록에 나타나지 않는지 확인한다

**Checkpoint**: 세 스토리 모두 동작한다.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 견고성(FR-006), 문서, 게이트. FR-006은 특정 스토리가 아니라 전 경로에 걸친 요구사항이라 여기에 둔다.

- [X] T024 [P] `fs_provider_session_repository.rs` 테스트 모듈에 T-6을 추가한다 — 메타 JSON이 깨진 세션과 정상 세션이 함께 있을 때 정상 세션만 반환되고 목록 전체가 실패하지 않는지 검증한다 ([contracts C-8](./contracts/provider-session-repository.md))
- [X] T025 [P] `fs_provider_session_repository.rs` 테스트 모듈에 T-7을 추가한다 — 로그에 깨진 줄이 섞여 있어도 유효 이벤트만 계수하고 세션 자체는 반환되는지 검증한다
- [X] T026 `docs/kiro-cli-session-resume.md`를 작성한다 — 지원 범위, 세션 저장소 구조, 빈 세션 제외 기준과 그 근거(`session_created_reason`을 쓸 수 없는 이유), `message_count` 근사 특성, `.jsonl` 부재 시 누락 가능성을 기술한다
- [X] T027 `cd apps/agentic-workbench/src-tauri && cargo test --lib`로 전체 회귀를 확인한다 — T001의 기준선 대비 기존 테스트가 하나도 깨지지 않아야 한다
- [X] T028 `cargo fmt --all --check`와 `cargo clippy --lib`를 통과시킨다 — 새로 생긴 clippy 경고가 없어야 한다
- [X] T029 [quickstart §3.5](./quickstart.md)를 수행해 손상된 메타 파일이 나머지 목록을 막지 않는지 실제 앱에서 확인한다 (실제 세션 파일은 백업 후 복구할 것)

---

---

## 실제 검증 방법 (실행 기록, 2026-08-25)

수동 검증 태스크(T016·T017·T020·T023·T029)는 quickstart에 적은 "앱 UI 조작"
대신 **더 결정적인 경로로 수행**했다. AW 창이 다중 모니터 환경에서 포커스를
안정적으로 잡지 못해 좌표 클릭이 엉뚱한 창으로 가는 문제가 있었고, UI 조작
자체는 이 기능의 검증 대상이 아니기 때문이다.

| 태스크 | 수행한 검증 | 결과 |
|---|---|---|
| T017 | `kiro-cli acp`에 `initialize` → `session/load`를 직접 전송 (AW의 `load_session_params`와 동일한 `{sessionId, cwd, mcpServers}` 형태) | `loadSession:true` 광고 확인, 에러 프레임 없음, **`session/update` 알림 307건 수신** — 기존 대화가 재생되어 맥락이 살아 있음이 확인됨 |
| T016·T020·T023 | 실제 저장소(`~/.kiro/sessions/cli`)를 대상으로 `FsProviderSessionRepository::list("kiro-cli", Path(...))`를 직접 호출 | 대상 cwd의 세션 **3건 반환**, 검증용으로 만든 빈 세션 1건은 **제외됨**. 제목·모델·`updated_at`(RFC3339) 모두 정상 |
| T029 | 실제 세션 3건을 임시 디렉토리에 복사한 뒤 메타 하나를 손상시키고 `KIRO_SESSION_DIR`로 조회 | **2건 반환** — 손상된 세션만 빠지고 목록 전체는 정상. 사용자의 실제 파일은 건드리지 않음 |

검증에 쓴 임시 테스트(`manual_real_kiro_sessions`)는 로컬 경로에 의존하므로
실행 후 제거했다. 커밋되는 테스트는 fixture 기반 9종뿐이다.

**남은 확인 항목**: 앱 UI에서 목록이 실제로 렌더링되는 모습은 사람이 눈으로
확인하는 편이 낫다. 백엔드가 올바른 데이터를 반환하는 것까지는 위와 같이
확인했고, 목록 UI는 provider에 무관한 기존 경로라 추가 위험은 낮다.

---

## Dependencies & Execution Order

### Phase 의존 관계

```
Phase 1 (Setup)
   ↓
Phase 2 (Foundational) ← 모든 스토리의 전제, 건너뛸 수 없음
   ↓
Phase 3 (US1, P1) ← MVP 경계
   ↓
Phase 4 (US2, P2) ← US1의 parse_kiro를 확장
   ↓
Phase 5 (US3, P3) ← US1의 parse_kiro를 확장
   ↓
Phase 6 (Polish)
```

### 스토리 간 의존

US1·US2·US3는 **모두 `parse_kiro` 하나를 확장**하는 구조라 파일 수준에서 순차적이다. 다만 각 스토리가 더하는 동작은 독립적으로 검증 가능하다:

- US1만 구현 → 목록이 뜨고 재개된다 (제목은 메타에 있을 때만, 빈 세션도 섞임)
- US1 + US2 → 제목 대체까지 동작
- US1 + US3 → 빈 세션 제외까지 동작 (US2 없이도 성립)

즉 US2와 US3는 서로 독립이며 순서를 바꿔도 된다.

### 태스크 수준 병렬 기회

`[P]`가 붙은 태스크는 서로 다른 테스트 함수를 추가하는 것이라 동시에 작성할 수 있다. 단 같은 파일(`fs_provider_session_repository.rs`)의 테스트 모듈을 건드리므로, 실제로 병렬 편집할 때는 충돌에 주의한다.

- Phase 3: T011, T012, T013, T014, T015 (T015만 다른 파일)
- Phase 6: T024, T025

구현 태스크(T008~T010, T018, T021)는 같은 함수를 순차적으로 확장하므로 병렬 불가다.

---

## Implementation Strategy

### MVP 범위

**Phase 1 + Phase 2 + Phase 3 (T001~T017)** 까지가 MVP다. 이 시점에 Kiro 세션이 목록에 뜨고 재개된다 — 기능의 존재 이유가 충족된다.

US2(제목 대체)와 US3(빈 세션 제외)는 목록 품질 개선이라 이후 증분으로 넘길 수 있다.

### 권장 순서

1. **T001**로 기준선을 기록한다 (회귀 판정에 필요)
2. **Phase 2를 한 번에** 완료한다 — 쪼개면 컴파일이 깨진 상태로 남는다
3. **T017을 최대한 일찍** 수행한다 ⚠️ — `session/load` 실동작이 미검증이라, 여기서 막히면 이후 작업(US2/US3)의 전제가 흔들린다. T008~T010 직후 목록이 뜨는 것만 확인되면 바로 재개를 시험해볼 것
4. 나머지는 우선순위 순으로 진행한다

### 리스크 대응

T017이 실패하는 경우 — 즉 Kiro가 `session/load`를 거부하는 경우 — US1의 "재개" 절반이 성립하지 않는다. 이때는:

1. 실패 응답과 로그(`$TMPDIR/kiro-log/kiro-chat.log`, `KIRO_LOG_LEVEL=debug`)를 기록한다
2. FR-009가 지켜지는지 확인한다 (사용자에게 실패가 전달되고 새 세션으로 진행 가능한지)
3. 목록 노출까지는 그대로 가치가 있으므로 남기고, 재개 실패는 별도 이슈로 분리한다
