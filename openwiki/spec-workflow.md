# 스펙 주도 개발 (SpecKit)

이 저장소는 **SpecKit**(v0.12.3.dev0) 기반의 스펙 주도 개발(SDD) 워크플로를 사용합니다. 모든 주요 기능은 `specs/` 디렉토리에 스펙 산출물을 남깁니다.

## 워크플로

`/.specify/workflows/speckit/workflow.yml`에 정의된 "Full SDD Cycle" (7단계):

```text
1. specify        사용자 설명에서 스펙 생성
2. review-spec    ↕ 인간 승인 게이트 (approve/reject)
3. plan           구현 계획 생성
4. review-plan    ↕ 인간 승인 게이트 (approve/reject)
5. tasks          태스크 목록 생성
6. implement      태스크 실행
```

`full`, `backend-only`, `frontend-only` 스코프를 지원합니다.

현재 활성 기능은 `.specify/feature.json`에 추적됩니다.

## 스펙 디렉토리 구조

각 기능은 `specs/NNN-기능명/` 디렉토리에 다음 파일들을 포함합니다:

| 파일 | 내용 |
|------|------|
| `spec.md` | 사용자 스토리 (P1/P2/P3 우선순위), 수용 시나리오, 엣지 케이스 |
| `plan.md` | 구현 계획: 기술 컨텍스트, 컨스티튜션 체크, 프로젝트 구조 |
| `research.md` | 배경 조사 및 기존 코드 분석 |
| `data-model.md` | 타입 정의 및 데이터 컨트랙트 |
| `contracts/` | API/UI 컨트랙트 문서 (여러 파일 가능) |
| `tasks.md` | 사용자 스토리별 단계적 태스크 목록 (`[P]` 병렬 마커, 정확한 파일 경로) |
| `quickstart.md` | 검증 명령과 테스트 단계 |
| `checklists/requirements.md` | 요구사항 검증 체크리스트 |

## 컨스티튜션 원칙

`/.specify/templates/constitution-template.md`가 모든 스펙에 적용되는 핵심 원칙을 정의합니다:
- 라이브러리 우선 (앱 코드보다 공유 라이브러리 우선)
- 테스트 우선 (TDD 필수)
- 모노레포 경계 규칙
- 통합 테스트
- 거버넌스

## 구현된 주요 스펙

`specs/` 디렉토리에는 19개 기능 디렉토리가 있습니다. 주요 스펙:

| 스펙 | 기능 |
|------|------|
| `001-worktree-auto-refresh` | Worktree 자동 새로고침 |
| `006-shared-worktree-changes` | 크로스앱 worktree 변경사항 공유 (`git-core` 추출) |
| `007-worktree-session-performance` | 세션 페이지 로딩 성능 최적화 |
| `008-agent-env-profiles` | 에이전트 프로필 + 환경변수 주입 |
| `011-mcp-session-ui-control` | MCP 기반 세션 UI 제어 (창 제목) |
| `012-queue-prompt-order` | 대기열 프롬프트 순서 보존 |
| `013-main-extra-agent-panels` | 메인 창 추가 에이전트 실행 패널 |
| `014-acp-tool-autocomplete` | ACP 툴 프롬프트 자동완성 |
| `016-mermaid-modal-preview` | Mermaid 모달 확대 미리보기 |
| `019-improve-permission-dialog` | 권한 다이얼로그 레이아웃 개선 |

## 기존 문서 (docs/)

`docs/` 디렉토리에 28개의 아키텍처 및 기능 설계 문서가 있습니다 (한국어, 영어 파일명). 주요 문서:

- `portable-architecture-plan.md` — AW의 ACP 워크벤치를 재사용 가능한 코어 + 호스트 어댑터로 추출하는 계획
- `git-worktree-changes-architecture.md` — GE와 AW 간 worktree 상태/diff 공유 아키텍처
- `ralph-mode-implementation.md` — Ralph Loop (자동 후속 프롬프트 반복) 구현
- `goal-feature-implementation.md` — worktree별 ThreadGoal 구현
- `acp-agent-command-override.md` — 에이전트 프로필 (명령어/env 오버라이드)
- `app-wide-acp-session-context-summary.md` — 앱 주입 MCP 서버로 worktree/목표/세션 요약 제공
- `dual-pane-agent-session-exchange-design.md` — 동일 worktree 내 듀얼 에이전트 세션 설계

기존 `docs/` 문서는 여전히 유용하므로, 이 위키는 그것을 요약하고 링크하는 역할을 합니다. 스펙 세부사항은 항상 해당 `specs/` 디렉토리와 `docs/` 문서를 직접 참조하세요.
