# Tasks: Markdown 작업 보드

**Input**: [[speckit-plan | 구현 계획]] · [[speckit-spec | 기능 명세]] · [[speckit-data-model | 데이터 모델]]

## Phase 1: Setup

- [x] T001 core package에 task fixture 추가
- [X] T002 작업 보드 page route 등록
- [ ] T003 Storybook 기본 story scaffold 생성

## Phase 2: Foundational

- [x] T004 `DocumentTaskSummary` 타입 구현
- [ ] T005 Markdown block에서 chapter 범위 계산
- [ ] T006 문서별 task summary selector 구현

## Phase 3: User Story 1 - 진행률 확인

**Goal**: 문서별 완료·미완료 task를 한눈에 확인한다.

- [ ] T007 [P] summary badge 컴포넌트 구현
- [ ] T008 [P] 빈 문서와 task 없는 문서 상태 구현
- [ ] T009 작업 보드 목록에 summary 연결
- [ ] T010 Storybook에서 light/dark 상태 검증

## Phase 4: User Story 2 - 원본 이동

- [ ] T011 task 행 클릭 interaction 구현
- [ ] T012 문서 열기와 block scroll 연결
- [ ] T013 누락된 파일 오류 상태 구현

## Phase 5: Polish

- [ ] T014 접근성 이름과 키보드 탐색 검증
- [ ] T015 MA 및 AW 회귀 테스트 실행
- [ ] T016 [[speckit-checklist | 요구사항 체크리스트]] 최종 검토

