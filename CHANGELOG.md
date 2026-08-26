# 변경 이력

이 프로젝트는 릴리스 빌드에 `YYYY.M.D` 형식의 CALVER를 사용합니다.

## 2026.8.1-rc.2 - 2026-08-27

### 추가

- Kiro CLI 에이전트의 모델·effort 선택과 기존 세션 목록·재개 지원
- 태그 기반 Apple Silicon macOS DMG 빌드·검증·게시 workflow
- pull request와 `main` 변경에 대한 TypeScript·Rust 품질 gate
- 릴리스 빌드 시 manifest를 수정하지 않고 버전과 About 정보를 주입하는 빌드 경로

### 수정

- permission 요청 결과가 별도 timeline 행으로 중복되던 문제
- 완료된 permission 뒤 오래된 대기 이벤트가 재전달될 때 상태가 되돌아가던 문제
- Rust 1.98 strict clippy gate에서 발견된 타입·lock 수명·경로 API 경고
- OpenWiki workflow가 provider 설정 없이 실행되어 매일 실패하던 문제

### 검증

- permission 승인·거부·취소·선택 없음, 다중 요청, run 격리, 재전달 회귀 테스트
- frozen pnpm 설치, TypeScript 검사, 전체 테스트, production build
- `cargo fmt`, strict clippy, Rust workspace 전체 테스트

## 2026.8.1-rc.1 - 2026-07-31

- Agentic Workbench 최초 Apple Silicon macOS Private RC
- 앱 전체 글자 크기 설정과 panel/window 상태 복원 안정화
- agent orchestration workspace와 worktree 작업 화면 개선
