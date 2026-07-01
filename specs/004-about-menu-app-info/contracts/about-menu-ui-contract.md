# UI Contract: About 메뉴 앱 정보 표시

## Scope

이 계약은 `agentic-workbench` 사용자가 native menu에서 About 항목을 선택했을 때 관찰해야 하는 사용자 경험을 정의한다.

## Entry Point Contract

### macOS

- About 항목은 앱 메뉴에서 발견 가능해야 한다.
- 메뉴 라벨은 `About Agentic Workbench` 또는 OS 관례상 동등하게 이해 가능한 About 라벨이어야 한다.

### Windows/Linux

- About 항목은 `Help` 메뉴 아래에서 발견 가능해야 한다.
- 메뉴 라벨은 `About Agentic Workbench` 또는 동등하게 이해 가능한 About 라벨이어야 한다.

## Dialog Contract

About 항목을 선택하면 작은 정보 dialog 또는 창이 열린다.

### Required Content

```text
Agentic Workbench

Version: <official-app-version-or-fallback>
Commit: <build-commit-hash-or-fallback>
```

### Required Behavior

- dialog는 읽기 전용이다.
- dialog는 사용자가 닫을 수 있어야 한다.
- dialog를 열거나 닫아도 현재 작업 화면과 진행 중인 세션 상태는 변경되지 않아야 한다.
- commit hash를 확인할 수 없으면 `Commit:` 라벨은 유지하고 값 위치에는 명확한 fallback 값을 표시해야 한다.
- dev build와 release build에서 `Version` 및 `Commit` 라벨과 표시 순서는 동일해야 한다.

## Non-Goals

- release notes 표시
- 라이선스 전문 표시
- commit hash 복사 버튼
- 앱 업데이트 확인
- 사용자 또는 프로젝트별 상태 표시
