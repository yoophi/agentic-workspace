# Implementation Plan: Markdown 작업 보드

**Branch**: `042-markdown-work-board` | **Spec**: [[speckit-spec]]  
**Related**: [[speckit-data-model | 데이터 모델]] · [[speckit-tasks | 구현 작업]]

## Summary

공용 Markdown parser에서 task 통계를 계산하고, 앱의 작업 보드가 결과를 읽어 문서별 진행률을 표시한다.

## Technical Context

| 항목 | 선택 |
|---|---|
| Language | TypeScript 5.x, React 19 |
| Parsing | `markdown-annotation-core` |
| UI | Feature-Sliced Design, shadcn/ui |
| Testing | Vitest, Storybook |

## Constitution Check

- 공용 parsing 규칙은 `packages/markdown-annotation-core`에 둔다.
- 화면 조합은 `pages`, 사용자 상호작용은 `features`에 둔다.
- 재사용 가능한 UI는 Storybook에 등록한다.

## Project Structure

```text
packages/markdown-annotation-core/src/tasks/
  summarize-document-tasks.ts
apps/markdown-annotator/src/
  entities/task-summary/
  features/open-task-source/
  pages/work-board/
```

## Delivery Flow

```mermaid
flowchart LR
  A[Markdown parse] --> B[Task summary]
  B --> C[Document model]
  C --> D[Work board]
  D --> E[Source navigation]
```

## Risk Review

가장 큰 위험은 editor와 preview가 서로 다른 task 범위를 사용하는 것이다. 공용 집계 함수와 fixture를 공유하여 차이를 방지한다.

