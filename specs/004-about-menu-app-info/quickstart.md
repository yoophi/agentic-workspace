# Quickstart: About 메뉴 앱 정보 표시 검증

## Prerequisites

- Rust/Cargo workspace가 빌드 가능한 상태여야 한다.
- pnpm 의존성이 설치되어 있으면 frontend typecheck도 실행한다.
- native menu 수동 확인을 위해 데스크톱 세션에서 앱을 실행할 수 있어야 한다.

## Static Validation

```bash
cargo fmt --package agentic-workbench
cargo check -p agentic-workbench
```

pnpm 의존성이 설치된 환경에서는 다음도 실행한다.

```bash
pnpm --filter @yoophi/agentic-workbench check-types
```

## Manual End-to-End Validation

1. 앱을 개발 모드 또는 빌드 산출물로 실행한다.

   ```bash
   pnpm --filter @yoophi/agentic-workbench tauri:dev
   ```

2. macOS에서는 앱 메뉴에서 `About Agentic Workbench`를 찾는다.
3. Windows/Linux에서는 `Help > About Agentic Workbench`를 찾는다.
4. About 항목을 선택한다.
5. dialog에 앱 이름, `Version`, `Commit`이 표시되는지 확인한다.
6. 표시된 `Version` 값이 `apps/agentic-workbench/package.json`의 `version`과 일치하는지 확인한다.
7. 표시된 `Commit` 값이 빌드 commit hash 또는 명확한 fallback 값인지 확인한다.
8. dialog를 닫은 뒤 현재 작업 화면과 진행 중인 세션 상태가 변경되지 않았는지 확인한다.

## Fallback Validation

commit hash를 확인할 수 없는 환경을 구성한 뒤 앱을 실행한다. About dialog는 계속 열려야 하며 `Commit` 값은 명확한 fallback 값으로 표시되어야 한다.

## Documentation Validation

`docs/native-about-menu-verification.md`에 다음 내용이 포함되어야 한다.

- 대상 앱과 기능 범위
- 표시 정보
- 메뉴 선택부터 dialog 표시까지의 흐름
- macOS 및 Windows/Linux 수동 검증 절차
- Rust 컴파일 검증과 frontend typecheck 조건
