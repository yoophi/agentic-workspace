# 보안 정책

## 지원 범위

정식 버전이 나오기 전까지 최신 `2026.8.1-rc.x` Agentic Workbench 릴리스 후보만 보안 수정을 받습니다. 과거 RC와 소스에서 직접 만든 임의 빌드는 지원 대상이 아닙니다.

## 취약점 신고

민감한 내용은 공개 이슈에 작성하지 마세요. GitHub의 [비공개 보안 권고 신고](https://github.com/yoophi/agentic-workspace/security/advisories/new)를 우선 사용하고, 해당 기능을 사용할 수 없으면 `yoophi@gmail.com`으로 재현 절차와 영향 범위를 보내 주세요.

다음 정보를 포함하면 확인이 빨라집니다.

- Agentic Workbench 버전과 About 화면의 commit·tag
- macOS 버전과 CPU 아키텍처
- 재현 절차와 기대·실제 동작
- 관련 로그에서 token, API key, 개인 경로를 제거한 최소 증거

## 보안 경계

Agentic Workbench는 로컬 worktree에서 외부 ACP adapter와 agent CLI를 실행합니다. permission 요청을 확인하지 않은 채 승인하지 말고, `DangerouslySkipAllPermissions`와 유사한 무제한 모드는 신뢰할 수 있는 저장소에서만 사용하세요. 릴리스 파일은 함께 제공되는 SHA-256과 대조해야 합니다.
