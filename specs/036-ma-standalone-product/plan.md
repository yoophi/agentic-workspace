# Implementation Plan: Markdown Annotator 독립 제품화

**Branch**: `036-ma-standalone-product` | **Date**: 2026-08-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/036-ma-standalone-product/spec.md`

## Summary

Markdown Annotator(MA)를 root 폴더의 Markdown 문서를 점진적으로 탐색하고, 문서별 annotation과 review 결정을 안전하게 보존하며, Markdown·JSON 피드백으로 내보내는 macOS 독립 제품으로 완성한다.

프런트엔드는 먼저 범용 file entry를 검색·정렬·압축 tree row로 변환하는 순수 `@yoophi/file-browser-core`를 만들고 AW를 첫 소비자로 전환한다. 두 앱의 tree interaction이 같은 계약으로 검증된 뒤 kit-independent `@yoophi/file-browser-react`와 앱별 UI adapter를 연결한다. MA의 root scan, watcher, review persistence, recent roots, CLI와 native shell은 앱 전용 hexagonal backend와 FSD feature로 유지한다. review session은 revision이 있는 aggregate를 app-data에 원자 저장하고, 고신뢰 단일 일치만 annotation을 자동 재결합한다.

## Technical Context

**Language/Version**: TypeScript 5.6+, React 19, Rust edition 2024

**Primary Dependencies**: Tauri 2, `@tauri-apps/api` 2.x, `@tauri-apps/plugin-dialog`, React Query 5(AW), Zustand 5(MA), Tailwind CSS 4, base-ui 기반 MA shadcn primitives, radix 기반 AW shadcn primitives, `notify` 6, Serde/serde_json, SHA-256 구현 crate, Vitest 4, Storybook 10

**Storage**: Tauri app-data의 세션별 versioned JSON, root/document index, 전역 settings, 최근 snapshot 5개, 7일 trash와 corrupt 격리 영역. 원본 폴더에는 sidecar를 만들지 않는다.

**Testing**: Vitest 기반 shared core·React interaction·app adapter 테스트, Storybook 접근성/대규모 tree 검증, Rust unit/tempdir repository/watcher 테스트, MA·AW cross-app typecheck/test/build, macOS clean-install acceptance와 성능 benchmark

**Target Platform**: macOS 공식 지원 Tauri desktop app. Windows/Linux compile 호환은 유지하되 release acceptance 대상은 아니다.

**Project Type**: pnpm/Cargo monorepo의 multi-window Tauri desktop app + cross-app TypeScript packages

**Performance Goals**: 10,000 filesystem entry/1,000 Markdown root에서 첫 batch 1초 이내, 전체 scan 5초 이내, tree search/sort/toggle 100ms 이내, 1MB 문서 500ms 이내 표시, 5MB 문서 처리 중 UI 응답 유지

**Constraints**: `.md`/`.markdown`과 strict UTF-8(BOM 허용)만 지원; 창당 canonical root 하나; root 밖/traversal/directory symlink 차단; 저장된 annotation 유실·오결합 0건; active review 자동 삭제 금지; app-data 100MB 관리 목표; 자동 telemetry/원문 편집/내장 예제/자동 updater 없음; 한국어 UI; watcher와 앱 orchestration은 공유 package에 넣지 않음

**Scale/Scope**: shared package 2개, AW file tree 소비 지점 2개, MA start/settings/about/annotator 화면, root별 window·watcher·scan session, 문서별 review aggregate, Markdown/JSON export v1, CLI `ma [file-or-directory]`, macOS signed/notarized app·DMG

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 범용 tree 계산과 검증된 tree interaction은 `packages/file-browser-core`, `packages/file-browser-react`에 두고 AW와 MA가 adapter로 소비한다. MA root session·watcher·persistence·native shell은 `apps/markdown-annotator`에 유지하며 앱 간 직접 import를 추가하지 않는다. Rust `file-access` 공통 crate는 이번 범위 밖이다.
- **Feature-Sliced Frontend Architecture**: PASS — route/provider 조립은 `app`, 시작·annotator·settings·about 화면은 `pages`, browsing/review/export/navigation은 `features`, file/review/preferences/build-info 모델과 API adapter는 `entities`, 앱 전용 adapter UI는 `shared`, 생성 primitive는 `components/ui`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS — MA는 전용 top-level `ports`를 도입하고 port를 domain에 중복 배치하지 않는다. 순수 모델은 `domain`, 유스케이스는 `application`, command/menu/window event는 `inbound`, filesystem/watcher/JSON/CLI/macOS adapter는 `infrastructure`에 둔다. command는 service에만 위임한다.
- **Shared Core Before Shared UI**: PASS — `file-browser-core`와 cross-app fixture를 먼저 만들고 AW를 첫 소비자로 전환한 뒤 MA adapter를 추가한다. React tree는 core 계약과 양쪽 interaction이 수렴한 뒤 분리하며 Tauri/query/store/app shell에 의존하지 않는다.
- **Atomic Cross-App Verification**: PASS — shared package 단계마다 package check/test와 AW·MA typecheck/test/build를 함께 닫는다. AW 일반/Markdown tree와 MA progressive tree를 같은 fixture로 검증한다.
- **Documentation and Storybook**: PASS — 기존 제품화·이식 문서를 갱신하고 MA 사용자/데이터/release 문서를 추가한다. shared tree organism과 AW/MA adapter wrapper에 loading, compressed, search, error, Unicode, large virtualized story와 a11y 검증을 추가한다.
- **Testing and Safety**: PASS — tree/anchor/migration/decision 규칙은 순수 fixture 테스트, JSON repository는 tempdir interruption·corruption·revision 테스트, filesystem은 canonical root/traversal/symlink/UTF-8/size 테스트, watcher는 trailing debounce·lifecycle 테스트를 계획했다.

**Post-design re-check**: PASS — [research.md](./research.md), [data-model.md](./data-model.md), [contracts](./contracts/), [quickstart.md](./quickstart.md)에 shared/app seam, persistence invariants, path safety와 cross-app 검증이 구체화되었으며 위반이나 미해결 clarification이 없다.

## Project Structure

### Documentation (this feature)

```text
specs/036-ma-standalone-product/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── file-browser-contract.md
│   ├── review-session-contract.md
│   ├── feedback-export-v1.schema.json
│   └── native-shell-cli-contract.md
├── checklists/
│   └── requirements.md
└── tasks.md                              # /speckit-tasks가 생성
```

### Source Code (repository root)

```text
packages/file-browser-core/
├── package.json
└── src/
    ├── index.ts
    ├── types.ts
    ├── create-file-browser-rows.ts
    ├── create-file-browser-rows.test.ts
    └── fixtures.ts

packages/file-browser-react/
├── package.json
└── src/
    ├── index.ts
    ├── types.ts
    ├── FileBrowserTree.tsx
    ├── FileBrowserTree.test.tsx
    └── use-tree-keyboard-navigation.ts

apps/agentic-workbench/src/
├── entities/worktree-file/
│   └── lib/file-browser-adapter.ts
└── features/worktree-workspace/
    ├── model/file-tree.ts                # 앱 orchestration helper만 유지
    └── ui/
        ├── file-browser-components.tsx
        └── worktree-workspace-panel.tsx

apps/markdown-annotator/src/
├── app/
│   ├── App.tsx
│   └── providers/
│       └── application-events-provider.tsx
├── pages/
│   ├── start/StartPage.tsx
│   ├── annotator/AnnotatorPage.tsx
│   ├── settings/SettingsPage.tsx
│   └── about/AboutPage.tsx
├── features/
│   ├── file-browser/
│   │   ├── model/use-file-browser.ts
│   │   └── ui/FileBrowserPanel.tsx
│   ├── document-navigation/
│   ├── review-session/
│   ├── feedback-export/
│   ├── external-document-actions/
│   └── data-management/
├── entities/
│   ├── file-browser/
│   ├── document/
│   ├── review-session/
│   ├── global-preferences/
│   └── build-info/
├── shared/ui/
│   ├── file-browser-components.tsx
│   ├── markdown-viewer-components.tsx
│   └── annotation-dialog-components.tsx
├── components/ui/
└── stories/

apps/markdown-annotator/src-tauri/src/
├── domain/
│   ├── file_browser.rs
│   ├── document_identity.rs
│   ├── review_session.rs
│   ├── global_preferences.rs
│   └── build_info.rs
├── ports/
│   ├── file_browser.rs
│   ├── review_session_repository.rs
│   ├── preferences_repository.rs
│   ├── clock.rs
│   └── native_shell.rs
├── application/
│   ├── file_browser_service.rs
│   ├── review_session_service.rs
│   ├── data_management_service.rs
│   ├── preferences_service.rs
│   └── launch_target_service.rs
├── inbound/
│   ├── file_browser_commands.rs
│   ├── review_commands.rs
│   ├── settings_commands.rs
│   └── native_menu.rs
├── infrastructure/
│   ├── fs_file_browser.rs
│   ├── fs_root_watcher.rs
│   ├── json_review_session_repository.rs
│   ├── json_preferences_repository.rs
│   ├── macos_native_shell.rs
│   └── sha256_fingerprint.rs
├── cli_launcher.rs
└── lib.rs

docs/
├── 20260801-markdown-annotator-productization-plan.md
├── 20260802-markdown-browser-migration-preparation.md
├── 20260802-shared-folder-browser-module-strategy.md
├── markdown-annotator-data-and-recovery.md
└── markdown-annotator-release.md
```

**Structure Decision**: shared tree는 entry→row라는 작은 interface 뒤에 path normalization, batch merge, directory 합성, 검색, natural sort, chain compression과 visible flatten을 숨긴다. 앱 adapter는 AW의 lazy query 또는 MA의 progressive scan 결과를 공통 entry로 바꾼다. MA backend는 기존 단일 파일 reader/watcher를 wrapper로 남기지 않고 root browser와 review aggregate service로 교체한다. storage·watcher·window registry는 제품별 수명과 event 의미가 달라 shared package로 승격하지 않는다.

## Implementation Approach

### 1. Shared file browser core와 AW 선행 전환

1. AW의 lazy entry fixture와 `packages/git-ui`의 chain compression·Unicode fixture를 공통 계약 입력으로 고정한다.
2. `createFileBrowserRows(entries, options)`가 entry validation, first-wins batch dedupe, file-only ancestor 합성, directory merge, search ancestor 유지, natural sort, compression과 flatten을 한 번에 수행한다.
3. 압축 directory row의 identity와 toggle path는 chain의 마지막 canonical relative path다. 표시 label만 `b/b1`이다.
4. AW adapter가 `WorktreeFileEntry`를 공통 entry로 바꾸고 일반 file tree와 Markdown tree를 순차 전환한다. lazy loading의 `loadedDirs`, query key, stale 판정과 “로드된 범위만 검색” 안내는 AW에 남긴다.
5. 기존 AW shallow tree helper/test는 adapter orchestration에 필요한 부분만 남기고 observable core 계약 테스트로 대체한다. Git diff tree 소비자 전환은 별도 범위다.

### 2. Shared React tree와 앱별 adapter

1. 공통 tree는 WAI-ARIA tree/treeitem, roving focus, Arrow/Home/End, expand/select, active row scroll과 virtualized window를 소유한다.
2. header, search/sort control, progress/error/empty 문구, 아이콘과 class는 앱 adapter가 소유한다. UI kit primitive 전체가 package interface로 새지 않도록 row render slot과 class contract를 사용한다.
3. AW Radix adapter와 MA base-ui adapter를 각각 제공한다. MA Storybook에 a11y addon을 추가하고 같은 fixture를 앱별 wrapper로 검증한다.
4. virtualization은 1,000 문서 tree에서도 DOM row 수가 viewport에 비례하게 하며 focus 대상이 window 밖이면 먼저 scroll한 뒤 focus한다.

### 3. MA root browser와 window lifecycle

1. launch target은 canonical root와 optional selected relative document로 정규화한다. directory, file, no-arg(cwd)가 같은 resolver를 사용한다.
2. window identity를 문서 hash에서 root identity로 교체하고 root당 창 하나를 보장한다. 기존 native document tab 연결은 제거하고 창 안에서 active document 하나와 history를 관리한다.
3. scan은 blocking worker에서 취소 가능한 scan id로 실행한다. `.md`/`.markdown`, 전역 exact-name exclusion, hidden directory 허용, directory symlink 차단과 내부 file symlink dedupe를 적용한다.
4. 100 entries 또는 50ms 단위 batch가 sequence와 ancestor directory를 포함해 emit된다. UI는 stale scan id/sequence를 무시한다.
5. strict UTF-8 read는 BOM만 제거하고 bytes/mtime/fingerprint를 반환한다. structured error code를 한국어 UI message로 mapping하며 raw filesystem error를 노출하지 않는다.

### 4. Root watcher와 외부 변경

1. window/root당 recursive watcher 하나를 registry에서 관리하고 root 교체·창 파괴 시 drop한다.
2. current leading-edge watcher를 trailing debounce/coalescing으로 교체한다. watcher rename pair를 신뢰하지 않고 rescan/snapshot diff의 힌트로 사용한다.
3. event는 root revision과 create/modify/remove/rename 후보를 한 batch로 제공한다. tree와 current document가 같은 event를 소비해 중복 reload를 막는다.
4. modify는 current document auto reload, tree change는 non-destructive rescan을 유발한다. 제외 설정으로 숨겨진 current document는 닫지 않고 excluded 상태로 남긴다.

### 5. Review aggregate와 app-data persistence

1. `ReviewSession` 전체가 저장 단위이며 schemaVersion, revision, document identity/fingerprint, annotation, decision, export preferences, reading position과 status를 가진다.
2. autosave는 expected revision을 사용한다. per-session serialization과 unique temp→sync→snapshot rotate→atomic rename→parent sync 순서로 stale write와 interruption을 방지한다.
3. 세션별 JSON과 index/settings를 분리한다. corrupt current는 격리한 뒤 newest valid snapshot을 시도하며 unknown future schema는 reset하지 않고 recoverable error로 반환한다.
4. 최근 snapshot 5개, trash 7일, app-data 100MB maintenance를 적용한다. expired trash와 오래된 snapshot부터 정리하고 active review는 삭제하지 않는다.
5. 문서 변경 시 기존 block id, exact selected text+context의 유일 후보 순서로 재결합한다. ambiguity는 conflict, 없음은 orphan, 문서 없음은 missing으로 보존한다.
6. rename/move는 같은 root의 동일 fingerprint 단일 후보만 proposal을 만들며 사용자 확인 후 identity를 변경한다.

### 6. MA frontend product flow

1. 앱 직접 실행은 start page만 표시한다. recent root/document 선택, folder/file open, 3단계 onboarding과 CLI status를 제공하며 내장 example selector/import를 제거한다.
2. annotator는 좌측 virtualized tree, 중앙 reader, 우측 review/export의 3영역으로 구성한다. TOC는 collapsible, 좌우 panel과 reading position은 root/document scope로 복원한다.
3. `use-file-browser`는 scan progress와 selected request token을 관리하고 `ReviewSessionStore` hydrate/save가 document switch 전에 닫히도록 한다.
4. annotation type을 change-request/question/note/delete로 제한하고 open/resolved, document draft/changes-requested/approved/stopped 전이를 적용한다.
5. internal Markdown link/wikilink는 같은 selection 진입점을 사용하고 root 밖 local path를 거부한다. HTTP/HTTPS는 검증된 external action으로 연다.
6. UI는 한국어로 통일하고 keyboard shortcut, VoiceOver tree semantics, narrow window overlay와 focus restoration을 검증한다.

### 7. Feedback export contract

1. 현재 문서의 open annotation을 기본 선택하고 resolved는 opt-in이다. annotation 없이 decision만 export할 수 있다.
2. Markdown은 사람이 읽을 수 있는 경로·결정·원문·comment를 제공한다. JSON v1은 document identity, fingerprint, decision, 선택 annotation과 schemaVersion을 deterministic하게 직렬화한다.
3. clipboard와 UTF-8 file save를 제공하고 clipboard 실패 시 file/direct selection fallback을 제공한다.
4. JSON Schema와 cross-app fixture를 안정 계약으로 관리한다. AW 직접 import/전송은 이번 범위에 넣지 않는다.

### 8. Native shell, Settings, About와 CLI

1. AW의 native menu 구성과 stable settings window 패턴을 MA-local로 적용한다. Settings는 exclusion names, font size, CLI/data management만 제공하며 theme는 제외한다.
2. About은 message dialog가 아니라 stable route/window로 만들고 product/build/local-first/license/notices/link를 표시한다.
3. Finder reveal은 검증된 문서에 `open -R`, 기본 앱은 `open`을 argument array로 실행한다. path copy는 검증된 canonical display path만 사용한다.
4. CLI는 `ma [file-or-directory]` wrapper 하나를 제품 계약으로 삼고 cold/single-instance가 같은 resolver를 사용한다. explicit install/check/reinstall/remove와 stale target 진단을 제공한다.
5. release CALVER는 build config overlay와 build env에 같은 값을 주입하고 manifest를 릴리스 때문에 변경하지 않는다. About, bundle metadata와 artifact name 일치를 검증한다.

### 9. Verification과 release

1. package/app/Rust 자동 검증을 cross-app matrix로 실행한다.
2. 10k entry/1k Markdown, 1MB/5MB document, watcher burst, corruption/migration/quota와 symlink security fixture를 검증한다.
3. keyboard-only와 VoiceOver acceptance, no-telemetry/redacted diagnostics를 검증한다.
4. CALVER RC app/DMG를 Developer ID로 서명, notarize, staple하고 Gatekeeper·clean account에서 install/CLI/Finder/data upgrade smoke를 통과시킨다.

## Complexity Tracking

> 위반 없음 — 해당 없음.
