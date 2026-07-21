# Phase 1 Data Model: Hushline Agent Run

엔티티는 (A) 공유 코어 `crates/acp-agent-core`가 이미 정의한 것과 (B) hushline이 신규로
소유하는 것으로 나뉜다. 코어 엔티티는 재사용(무변경)이 원칙이다.

## A. 공유 코어 엔티티 (crates/acp-agent-core, 재사용)

### AgentRunRequest
- **의미**: agent run 시작 요청. hushline이 채워서 코어에 전달.
- **주요 필드**: `goal`(실행 목표 텍스트), `agent_id`, `cwd`(자막 결과 폴더),
  `agent_command`/`agent_env`(해석된 실행 명령·환경), `permission_mode`, `run_id`,
  `mcp_servers`(본 기능에선 비움 — MCP 서빙은 범위 밖), `resume_*`(미사용).
- **검증**: `goal` 비어있지 않음; `cwd`는 앱 관리 경로 하위(경로 검증); 동시 run 상한 준수.

### RunEvent (스트리밍 이벤트)
- **의미**: run 진행 중 발생하는 이벤트. hushline UI가 소비.
- **본 기능이 사용하는 variant**: `Lifecycle{status,message}`, `AgentMessage{text}`(정리 결과/
  답변 본문), `Thought{text}`, `Tool{...}`·`Permission{...}`(부작용 도구 사용 시),
  `Usage`, `Error{message}`. (`RalphLoop`, `Terminal` 등은 미사용)
- **직렬화 계약**: `#[serde(rename_all="camelCase")]` — TS `RunEventEnvelope`와 1:1.

### AgentRun
- **의미**: 실행 단위 스냅샷(id, goal, 상태). registry가 소유·추적.
- **상태 전이**: Started → SessionCreated → PromptSent → (PromptCompleted | Cancelled | Error).

### PermissionMode
- **값**: Default, Auto, ReadOnly, Plan, AcceptEdits, DangerouslySkipAllPermissions.
- **본 기능 기본값**: ReadOnly/Plan으로 시작, 부작용 필요 시 사용자 승격.

### 소유/동시성 (registry)
- run 소유자: generic string(세션/창 식별자). 창 종료 시 소유 run 취소.
- 동시 run 상한: env(`ACP_MAX_RUNS`)로 설정.

## B. Hushline 신규 엔티티 (apps/hushline, 앱 고유)

### Transcript (기존 · 입력 원본)
- **의미**: 기존 자막 결과. 정리·대화의 입력.
- **필드(기존)**: `url`, `title`, `transcript`, `transcript_path`, `language`, `model`,
  `json_path`, `cached` 등.
- **관계**: OrganizedDocument·ChatSession이 이 항목을 참조.

### OrganizedDocument (신규)
- **의미**: 자막을 사용자가 지정한 방식으로 재구성한 새 문서.
- **필드**: `id`, `source_transcript_id`(또는 url), `style`(정리 방식/지시), `title`,
  `content`(수집된 AgentMessage 본문), `created_at`, `path`(저장 위치).
- **검증**: `content` 비어있지 않을 때만 저장; `path`는 앱 관리 출력 폴더 하위; UTF-8; 크기 상한.
- **관계**: 하나의 Transcript에 여러 OrganizedDocument 가능(N:1).

### ChatSession (신규)
- **의미**: 특정 문서를 대상으로 한 세션 내 다회 대화.
- **필드**: `id`, `document_id`(또는 transcript 참조), `run_id`(코어 AgentRun 연결),
  `messages[]`({role, text, created_at}), `created_at`.
- **상태**: active(진행 중 run 존재) → closed(창/앱 종료 시 run 정리, 로그는 유지).
- **검증**: `run_id` 소유 범위(세션/창) 일치; 세션 지속성(재개)은 범위 밖.
- **관계**: 하나의 OrganizedDocument(또는 Transcript)에 여러 ChatSession 가능(N:1).

## 저장 위치

- OrganizedDocument·ChatSession은 기존 자막 결과와 동일하게 앱 관리 출력 폴더의 구조화 JSON으로
  영속(경로·크기·UTF-8 검증). ACP 세션 자체의 재개용 영속은 사용하지 않음(noop store).

## 관계 요약

```text
Transcript 1 ──< OrganizedDocument 1 ──< ChatSession
     └──────────────< ChatSession (문서 없이 자막 직접 대화도 허용)
ChatSession ── run_id ──> AgentRun(코어, registry 소유)
```
