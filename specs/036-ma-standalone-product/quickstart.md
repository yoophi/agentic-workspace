# Quickstart: Markdown Annotator 독립 제품화 구현 검증

## 목적

이 문서는 구현 순서와 feature 완료 시 반드시 통과해야 할 cross-app 검증을 정의한다. 아래 명령은 현재 workspace의 실제 package 이름과 script를 기준으로 한다.

## 구현 순서

1. `file-browser-core` public contract와 공통 fixture를 작성한다.
2. AW의 일반/Markdown file tree를 adapter로 전환하고 기존 동작을 고정한다.
3. `file-browser-react`의 tree semantics, keyboard와 virtualization을 작성해 AW에 연결한다.
4. MA backend의 root resolver, safe scan/read, progressive event와 watcher를 구축한다.
5. MA review aggregate repository, migration/recovery/retention을 구축한다.
6. MA start/file browser/reader/review/export UI와 shared tree adapter를 연결한다.
7. native menu, Settings/About, Finder/default app와 CLI lifecycle을 연결한다.
8. CALVER RC를 서명·notarize하고 clean macOS acceptance를 수행한다.

단계 1~3에서 AW 회귀를 먼저 닫기 전 MA 전용 요구를 shared core에 추가하지 않는다. watcher, scan과 review persistence는 shared package로 이동하지 않는다.

## 개발 환경 확인

```bash
pnpm install
pnpm --filter @yoophi/file-browser-core test
pnpm --filter @yoophi/file-browser-react test
```

package가 처음 생성되는 단계에서는 root workspace 설정과 package naming convention을 기존 `packages/*`에서 확인한다.

## Cross-app 자동 검증

각 shared 변경마다 다음 범주를 모두 실행한다.

```bash
pnpm --filter @yoophi/file-browser-core test
pnpm --filter @yoophi/file-browser-react test
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/markdown-annotator check-types
cargo test --manifest-path apps/markdown-annotator/src-tauri/Cargo.toml
```

shared package를 수정한 PR에서 AW 또는 MA 검증을 후속 작업으로 미루지 않는다.

## 핵심 fixture

### Tree

```text
a/file.md
b/b1/file2.md
c/non-markdown.txt
d/file.md
```

MA 결과에는 `a`, `b/b1`, `d`가 나타나고 `c`는 나타나지 않아야 한다. `b/b1` toggle identity는 `b/b1`이어야 한다. AW adapter는 같은 core fixture에서 비-Markdown scope도 구성할 수 있어야 한다.

### 경로 안전성

- absolute path, `../`, NUL, root 밖 symlink는 거부
- directory symlink는 내부를 포함해 follow하지 않음
- root 내부 regular Markdown file symlink는 canonical target 기준 dedupe
- `.md`, `.markdown`, UTF-8/BOM만 성공
- unreadable branch는 warning 후 나머지 scan 계속

### Persistence

- process interruption 전/후 current JSON이 valid
- stale revision 저장이 current를 덮어쓰지 않음
- corrupt current에서 최신 valid snapshot 복구
- unknown future schema가 삭제/초기화되지 않음
- snapshot 5개, trash 7일, 100MB maintenance 순서
- active review는 quota 정리에서 유지

### Reconciliation

- block ID exact 단일 match는 attached
- text+context exact 단일 match는 attached
- 반복 문구는 conflict
- 일치 없음은 orphan
- 파일 없음은 missing
- same-root identical fingerprint 단일 후보도 confirm 전에는 relink하지 않음

## Storybook과 접근성

shared tree organism과 AW/MA wrapper에 loading, partial scan, compressed path, search, empty, permission warning, Unicode, 1,000-document virtualized story를 둔다. keyboard-only로 모든 row 이동·expand·select가 가능하고 VoiceOver가 level/expanded/selected를 읽는지 확인한다.

## 성능 acceptance

- 10,000 filesystem entries/1,000 Markdown: 첫 batch 1초, 전체 scan 5초
- 검색/정렬/toggle: action 후 100ms
- 1MB 문서: 500ms 안에 readable
- 5MB 문서 parsing 중 input 응답 유지

측정 fixture와 machine 정보, release/debug build 여부를 결과에 함께 기록한다.

## 제품 흐름 smoke test

1. 앱 아이콘 실행 시 이전 root 대신 시작 화면이 표시된다.
2. 폴더를 열면 첫 결과가 점진적으로 보이고 empty branch가 숨겨진다.
3. 문서를 선택해 annotation을 작성하고 다른 문서 왕복/재실행 후 복원한다.
4. 외부 editor로 수정·rename·삭제하여 attached/conflict/orphan/missing과 confirm relink를 확인한다.
5. open/resolved 선택과 decision을 Markdown/JSON으로 clipboard/file export한다.
6. 내부 link/wikilink, HTTP link, root 밖 local link 차단을 확인한다.
7. Finder 표시, 기본 앱 열기, 경로 복사를 확인한다.
8. Settings에서 제외 이름 변경과 data 삭제/복구, CLI 설치/확인/재설치/제거를 확인한다.
9. About의 CALVER/commit/tag/license/notices와 no-telemetry 문구를 확인한다.

## macOS release acceptance

RC build에만 `YYYY.M.D-rc.N`을 주입한다. app/DMG/About의 version과 commit을 비교하고 codesign verification, notarization, stapling, Gatekeeper 검사를 수행한다. clean user account에서 DMG 설치, 첫 실행, 폴더/파일 open, CLI와 수동 업그레이드 후 기존 review migration을 검증한다.

## 완료 조건

spec의 SC-001~SC-012 증거가 자동 test, benchmark 또는 기록된 macOS acceptance 중 하나에 연결되어야 한다. built-in example browser, 원문 편집, 자동 updater, AW 직접 전송은 산출물에 없어야 한다.
