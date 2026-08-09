# Contract: Native Shell and `ma` CLI

## Launch target

모든 진입점은 하나의 resolver를 사용한다.

```text
resolve(input?) -> CanonicalRoot + Optional<RelativeMarkdownDocument>
```

- 인자 없음: 호출 process의 current directory를 root로 연다.
- directory 하나: 해당 directory를 root로 연다.
- `.md`/`.markdown` 파일 하나: 부모를 root로 열고 파일을 선택한다.
- 여러 인자, glob 확장 결과 여러 개, stdin/headless option, 지원하지 않는 파일은 usage 오류다.
- canonical root가 이미 열려 있으면 새 창 대신 기존 창을 focus하고 optional document를 선택한다.

앱 아이콘 직접 실행은 CLI의 무인자 규칙을 쓰지 않고 start page만 표시한다.

## CLI lifecycle

Settings에서 다음 명시적 action을 제공한다.

- 상태 확인: `~/.local/bin/ma`의 존재, executable, 예상 app target 일치 여부
- 설치: directory를 안전하게 만들고 app bundle의 versioned launcher를 연결/작성
- 재설치: MA가 관리하는 기존 launcher임을 확인한 뒤 교체
- 제거: MA가 관리하는 정확한 launcher만 제거

관리자 권한을 요구하지 않는다. shell profile을 자동 수정하지 않으며 PATH에 없으면 사용자가 추가할 명령을 보여준다. 다른 파일을 덮어쓰거나 삭제하지 않는다.

## External document actions

backend는 현재 root/session에 속하는 canonical regular file임을 매 action마다 확인한다.

- Finder 표시: executable `/usr/bin/open`, arguments `[-R, canonicalPath]`
- 기본 앱으로 열기: executable `/usr/bin/open`, arguments `[canonicalPath]`
- 경로 복사: 검증된 canonical display path를 clipboard로 반환

shell 문자열, frontend 임의 executable, root 밖 경로를 받지 않는다. 파일이 이동/삭제되었으면 구조화된 오류와 재스캔 action을 제공한다.

## Native windows and menu

- root window label은 root ID 기반이며 canonical root당 하나다.
- Settings와 About은 각각 stable singleton label을 사용한다.
- 메뉴 action은 현재 focus root가 없어도 Settings/About/Open을 실행할 수 있다.
- About은 CALVER, commit, tag, 지원 형식, local-first/telemetry 정책, license/notices와 검증된 HTTPS link를 표시한다.

## Release metadata

release script가 CALVER를 Tauri config overlay와 compile-time build info에 동시에 주입한다. `package.json`, `Cargo.toml`, `Cargo.lock`, 기본 `tauri.conf.json`은 release version 때문에 수정하지 않는다. 다음 값은 artifact 검사에서 같아야 한다.

- app bundle short version
- DMG/artifact name의 CALVER
- About CALVER
- embedded commit/tag

stable 배포는 Developer ID signing, notarization, stapling과 `spctl` Gatekeeper 검증을 모두 통과해야 한다.
