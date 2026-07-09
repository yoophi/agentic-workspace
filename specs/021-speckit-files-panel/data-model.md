# Data Model: Speckit Files Panel

## SpeckitFeature

현재 worktree의 `specs` 영역 아래에 있는 하나의 Speckit 기능 단위이다.

**Fields**:
- `id`: 기능을 안정적으로 식별하는 root 기준 상대 경로
- `name`: 사용자에게 표시할 기능 폴더 이름
- `relativePath`: `specs/<feature>` 형태의 root 기준 상대 경로
- `documents`: 기능에 속한 `SpeckitDocument[]`
- `taskProgress`: `TaskProgressSummary | null`
- `status`: `ready | partial | error`

**Validation rules**:
- `relativePath`는 `specs/`로 시작해야 한다.
- 기능은 하나 이상의 표시 가능한 Speckit 문서를 가져야 목록에 표시된다.
- 같은 basename 문서가 있어도 `relativePath` 기준으로 구분한다.

## SpeckitDocument

기능에 속한 개별 Speckit 산출물이다.

**Fields**:
- `id`: root 기준 상대 경로
- `featureId`: 소속 `SpeckitFeature.id`
- `type`: `spec | plan | tasks | research | dataModel | quickstart | contract | checklist | other`
- `label`: 사용자 표시 이름
- `relativePath`: root 기준 문서 상대 경로
- `group`: `core | contracts | checklists | other`
- `size`: 파일 크기
- `modifiedMs`: 마지막 수정 시각
- `readState`: `unknown | readable | unreadable`
- `errorMessage`: 읽기 또는 분류 오류가 있을 때 사용자에게 표시할 짧은 메시지

**Validation rules**:
- root 밖 경로, absolute path, parent-dir path는 허용하지 않는다.
- 표시 대상은 markdown 문서로 제한한다.
- `contracts/*`와 `checklists/*` 문서는 기능 문서 아래 별도 group으로 표시한다.

## TaskProgressSummary

`tasks.md`에서 계산한 체크박스 작업 진행 요약이다.

**Fields**:
- `total`: 전체 체크박스 작업 수
- `completed`: 완료 체크박스 작업 수
- `remaining`: 미완료 체크박스 작업 수
- `state`: `noTasks | notStarted | inProgress | complete`
- `sourcePath`: 요약의 기준이 된 `tasks.md` 상대 경로

**Validation rules**:
- markdown checkbox task만 계산 대상이다.
- `completed + remaining`은 항상 `total`과 같아야 한다.
- `total`이 0이면 진행률 숫자 대신 `noTasks` 상태를 표시한다.

**State transitions**:
- `noTasks`: `total = 0`
- `notStarted`: `total > 0` and `completed = 0`
- `inProgress`: `total > 0` and `0 < completed < total`
- `complete`: `total > 0` and `completed = total`

## SpeckitPanelState

Speckit 패널의 사용자-visible 탐색 상태이다.

**Fields**:
- `selectedFeatureId`: 현재 강조된 기능
- `selectedDocumentPath`: markdown viewer로 열 대상 문서
- `loadState`: `idle | loading | ready | empty | error`
- `features`: `SpeckitFeature[]`
- `errorMessage`: 전체 목록 로딩 실패 시 표시할 메시지

**Validation rules**:
- 빈 `specs` 또는 없는 `specs`는 `empty` 상태이며 오류가 아니다.
- 개별 문서 읽기 실패는 가능한 경우 `SpeckitDocument.errorMessage`로 제한한다.
- `selectedDocumentPath`는 현재 worktree root 기준 상대 경로여야 한다.

## Relationships

- 하나의 `SpeckitFeature`는 0개 이상의 `SpeckitDocument`를 가진다. 표시 목록에는 1개 이상 문서가 있는 feature만 포함한다.
- 하나의 `SpeckitFeature`는 최대 하나의 `TaskProgressSummary`를 가진다.
- `TaskProgressSummary.sourcePath`는 해당 feature의 `tasks` type 문서와 일치해야 한다.
- `SpeckitPanelState.selectedDocumentPath`는 기존 markdown viewer가 읽을 `WorktreeTextFile.relativePath`와 같은 값이다.
