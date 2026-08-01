---
type: Development Workflow
title: Spec-driven development with SpecKit
description: The repository's SpecKit workflow, artifact structure, and recent specifications for Agentic Workbench workspace, orchestration, rendering, and appearance work.
tags: [speckit, specifications, development-workflow]
---

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

`specs/` 디렉토리의 최신 기능 스펙 중 다음 항목은 최근 AW workspace와 Markdown renderer 변경을 추적합니다:

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
| `027-agent-run-minimap` | 에이전트 실행 미니맵 |
| `028-collapsible-resizable-panel` | 접기/펼치기 가능한 크기 조절 패널 (`packages/ui`) |
| `029-ma-spec-markdown-preview` | SpecKit Markdown 프리뷰 + 주석 (wikilink 해석, TOC task 카운트) |
| `030-hushline-monorepo-integration` | hushline 모노레포 편입 + agent run 기능 (`acp-agent-core`, `@yoophi/agent-client` 추출) |
| `031-hide-workspace-panels` | worktree별 workspace panel 선택·크기 저장과 전체 숨김 |
| `031-sdd-workflow-controls` | SpecKit stage 제어와 Tasks/Kanban 보기 |
| `032-agent-run-tiles` | stable agent-run slot의 탭/타일 projection과 panel 교환 |
| `033-agent-orchestration` | Main Coordinator, durable read-only child task, activity rail, recovery |
| `033-markdown-rendering-quality` | CommonMark/GFM AST 기반 block/annotation 품질과 AW 주석 영역 전환 |
| `034-fix-list-render-order` | 분리된 목록의 원문 순서 보존 및 custom marker 규칙 |
| `035-adjust-font-size` | 모든 AW WebView의 영속적 글꼴 크기 단계 |

## 기존 문서 (docs/)

`docs/` 디렉토리에는 아키텍처 및 기능 설계 문서가 있으며, 수량은 빠르게 변하므로 특정 개수를 기준으로 삼지 않습니다. 주요 문서:

- `portable-architecture-plan.md` — AW의 ACP 워크벤치를 재사용 가능한 코어 + 호스트 어댑터로 추출하는 계획
- `git-worktree-changes-architecture.md` — GE와 AW 간 worktree 상태/diff 공유 아키텍처
- `ralph-mode-implementation.md` — Ralph Loop (자동 후속 프롬프트 반복) 구현
- `goal-feature-implementation.md` — worktree별 ThreadGoal 구현
- `acp-agent-command-override.md` — 에이전트 프로필 (명령어/env 오버라이드)
- `app-wide-acp-session-context-summary.md` — 앱 주입 MCP 서버로 worktree/목표/세션 요약 제공
- `dual-pane-agent-session-exchange-design.md` — 동일 worktree 내 듀얼 에이전트 세션 설계
- `markdown-annotator-preview.md` — SpecKit Markdown 프리뷰 + 주석 워크플로 설계
- `20260721-acp-agent-core-reuse-strategy.md` — ACP agent 실행 계층을 공유 crate로 추출하는 전략 및 결정
- `agent-orchestration-workspace.md` — Main Coordinator/Child product model, durable recovery, MCP capability 경계
- `markdown-rendering-quality.md` — AST block 품질, 목록 순서, annotation rail visibility
- `20260801-ma-aw-markdown-feature-comparison.md` — MA의 범용 문서 검토와 AW의 worktree/SpecKit workflow 경계

기존 `docs/` 문서는 여전히 유용하므로, 이 위키는 그것을 요약하고 링크하는 역할을 합니다. 스펙 세부사항은 항상 해당 `specs/` 디렉토리와 `docs/` 문서를 직접 참조하세요.
