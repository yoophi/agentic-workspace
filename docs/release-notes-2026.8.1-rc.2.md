# Agentic Workbench 2026.8.1-rc.2

Apple Silicon macOS용 두 번째 Private RC입니다. RC1 뒤 앱 동작 변경과 permission 중복 수정이 포함되어 stable 승격 전에 별도 후보로 검증합니다.

## 주요 변경

- permission 승인 결과를 원래 요청 timeline 항목에 병합합니다.
- 승인·거부·취소·선택 없음, 다중 요청, run 격리와 재전달 순서를 회귀 테스트로 고정했습니다.
- Codex 실행 모델·effort 선택과 Kiro CLI 새 세션·재개 세션을 지원합니다.
- Rust strict clippy gate와 태그 기반 Apple Silicon DMG workflow를 추가했습니다.

## 지원 환경

- Apple Silicon Mac
- macOS 11.0 이상
- Git
- ACP adapter 실행을 위한 Node.js/npm(`npx`)과 네트워크
- 사용하는 Codex, Claude Code 또는 Kiro CLI의 로컬 인증

## 설치

DMG를 열고 `Agentic Workbench.app`을 Applications로 복사합니다. 이 Private RC는 Developer ID 공증 전 ad-hoc 서명 빌드이므로 macOS가 차단하면 시스템 설정의 개인정보 보호 및 보안에서 사용자가 직접 허용해야 합니다.

다운로드한 DMG는 함께 첨부된 `.sha256` 파일과 대조하세요.

```sh
shasum -a 256 -c Agentic.Workbench_2026.8.1-rc.2_aarch64.dmg.sha256
```

## Rollback

앱을 완전히 종료하고 RC1 DMG의 앱으로 교체합니다. 교체 전에 `~/Library/Application Support/com.yoophi.agentic-workbench`를 별도 위치에 복사해 project, prompt, session, layout 데이터를 보존하세요. 앱 번들만 교체하고 데이터 디렉터리는 자동으로 삭제하지 않습니다.

## Known Issues

- Developer ID 서명·공증·stapling과 Gatekeeper stable 검증은 아직 완료되지 않았습니다.
- 자동 업데이트를 제공하지 않습니다.
- `pi-coding-agent`, `opencode`, Ralph Loop와 복잡한 child orchestration은 Experimental입니다.
- Codex, Claude Code, Kiro CLI의 설치·인증·네트워크 실패는 외부 runtime 상태에 따라 발생할 수 있습니다.
- production build의 대형 JavaScript chunk는 후속 성능 개선 대상입니다.
