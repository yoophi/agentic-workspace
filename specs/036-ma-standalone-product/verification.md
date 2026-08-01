# Verification: Markdown Annotator 독립 제품화

## Foundation

검증 환경: macOS, 2026-08-02, 개발 build

| 대상 | 명령 | 결과 |
|---|---|---|
| 앱 경계 | `pnpm check:app-boundaries` | PASS |
| file-browser-core | `pnpm --filter @yoophi/file-browser-core test` | PASS, 9 tests |
| file-browser-react | `pnpm --filter @yoophi/file-browser-react test` | PASS, 3 tests |
| AW adapter/UI | `pnpm --filter @yoophi/agentic-workbench test` | PASS, 401 tests |
| MA frontend | `pnpm --filter @yoophi/markdown-annotator test` | PASS, 17 tests |
| AW/MA types | 각 package `check-types` | PASS |
| MA backend | `cargo test --manifest-path apps/markdown-annotator/src-tauri/Cargo.toml` | PASS, 31 tests |
| AW production build | `pnpm --filter @yoophi/agentic-workbench build` | PASS |
| MA production build | `pnpm --filter @yoophi/markdown-annotator build` | PASS |

검증된 foundation 계약:

- 공통 core가 ancestor 합성, 중복 batch, natural sort, 상대 경로 검색과 directory chain 압축을 처리한다.
- AW는 adapter에서 기존 lazy load, 표시 순서와 비압축 정책을 유지한다.
- shared React tree는 WAI-ARIA, roving focus, keyboard 이동과 viewport 비례 DOM window를 제공한다.
- MA launch target은 directory/file/cwd를 canonical root로 정규화하고 `.md`/`.markdown`만 선택 문서로 허용한다.
- root 밖 traversal, 비지원 확장자와 invalid UTF-8 read를 거부한다.
- root ID 창 하나를 재사용하며 native document tab을 생성하지 않는다.

## US1

- 10,000개 entry의 path 검색과 tree 생성 fixture: 100ms 예산 이내 PASS.
- MA root scan은 blocking worker에서 실행되고 100-entry batch, scan ID, sequence와 completion을 전달한다.
- Markdown extension/exclusion/traversal/directory symlink/UTF-8 BOM fixture가 PASS했다.
- keyboard tree selection, compressed path, empty/partial-warning UI test가 PASS했다.

## US2

- ReviewSession annotation ID/group validation과 승인 확인 전이 Rust test가 PASS했다.
- grouped annotation CRUD, open/resolved와 승인 경고 frontend store test가 PASS했다.
- Annotation 작성 UI, review decision panel과 Storybook 상태를 등록했다.

## US3

- JSON repository의 unique temp, fsync, 원자적 rename, parent sync, expected revision 충돌, v0→v1 migration, future schema 거부, 손상 snapshot 복구와 최신 5개 보존 test가 PASS했다.
- 문서 전환 중 stale hydrate 폐기, 세션별 autosave 직렬화와 revision conflict 재조회 test가 PASS했다.
- root view 상태와 문서별 reading position repository를 연결했다.
- MA frontend 33 tests 및 typecheck, MA backend 20 tests가 PASS했다.

## US4

- root별 recursive watcher, trailing debounce, monotonic revision 및 stale event 거부 fixture를 구현했다.
- exact unique text만 자동 연결하고 중복은 conflict, 부재는 missing으로 분류하며 fingerprint rename은 후보 하나일 때만 제안하는 Rust test가 PASS했다.

## US5

- 현재 세션 범위의 deterministic JSON v1/Markdown export와 decision-only fixture가 PASS했다.
- open annotation 기본 선택, resolved opt-in, preview/copy/save 및 clipboard 실패 안내를 구현했다.

## US6

- directory/file/cwd resolver, single-instance root focus, canonical path containment과 `.md`/`.markdown` 제한 test가 PASS했다.
- Finder `open -R`, 기본 앱 `open`, 검증된 경로 복사 및 HTTP/HTTPS 외부 링크 command를 구현했다.
- `~/.local/bin/ma` launcher는 ownership marker가 없는 기존 파일을 덮어쓰거나 삭제하지 않는다.
- 실제 debug `.app`의 Settings에서 CLI 설치·재설치·제거를 실행했다. 설치 상태와 제거 상태가 즉시 갱신됐고, app-owned `~/.local/bin/ma`가 실행 가능 상태로 생성된 뒤 제거됐다.
- 생성된 launcher가 인자 없는 호출을 `$PWD`로 변환하는 것을 shell trace와 Rust 회귀 test로 확인했다. 콜드 스타트에서 directory 인자는 해당 root의 11개 Markdown 문서를 점진 표시했고, file 인자는 부모 root와 `spec.md`를 선택해 정상 렌더링했다.
- 실제 문서에서 경로 복사는 canonical `spec.md` 경로를 clipboard에 기록했고, Finder는 해당 파일을 선택했으며 기본 앱(Marked)이 열렸다.
- 별도 macOS 계정 또는 격리된 clean HOME에서의 반복 검증은 아직 실행하지 않았으므로 T078의 clean-account gate는 유지한다.

## US7

- versioned 전역 설정, exact directory-name/font-size validation, revision broadcast와 기본값 복원을 구현했다.
- review data trash, 7일 정리, snapshot/trash 우선 100MB quota 기반과 설정 UI를 구현했다. Theme control은 포함하지 않았다.
- 설정 exact-name/default validation, root rescan event 연결, live session을 보존하는 quota fixture와 frontend production build smoke가 PASS했다.

## US8

- launch target이 없으면 root를 복원하지 않고 시작 화면만 표시하며 built-in example browser를 제거했다.
- 시작 화면의 folder/file action과 3단계 안내, About의 CALVER/commit/tag/local-first/no-telemetry/license/notices test가 PASS했다.
- Settings/About singleton window와 native menu를 구현했다.
- 시작/About static UI test와 build-info/privacy code-path smoke가 PASS했다.
- 실제 debug `.app`에서 시작 화면, native Settings/About 메뉴와 singleton 창을 열었다. About은 version, commit, tag, MIT, notices와 local-only/no-telemetry 설명을 표시했고 Settings는 제외 디렉터리, 글꼴, CLI 및 데이터 관리 control을 노출했다.

## 전체 회귀

| 대상 | 결과 |
|---|---|
| file-browser-core | 10 tests, typecheck PASS |
| file-browser-react | 3 tests, typecheck PASS |
| AW | 401 tests, typecheck, production build PASS |
| MA frontend | 37 tests, typecheck, production build PASS |
| MA Rust | 31 tests 및 doc-tests PASS |
| app boundary | app-to-app import 없음, PASS |

SC-002~SC-004 10,000-path Rust release benchmark는 정렬·검색 3.37ms로 1초 gate를 통과했다. TypeScript benchmark fixture도 Vitest bench에서 실행됐다.

실제 debug `.app`의 접근성 트리에서 시작 화면, Settings/About, 파일 트리, 목차, review와 native shell action의 이름·역할을 확인했고 Tab으로 toolbar에서 tree까지 포커스가 이동했다. 실제 VoiceOver 음성 검증과 annotation 작성·export를 포함한 keyboard-only 완주 검증은 아직 실행하지 않았다.

실물 서명 identity와 Apple 공증 credential이 필요한 notarization/stapling/Gatekeeper는 이 개발 환경에서 실행하지 않았다. `security find-identity`에는 Apple Development identity만 있고 Developer ID Application identity가 없으며, `notarytool` keychain profile도 없다. 해당 항목과 clean-account acceptance는 release gate로 남긴다.

## SC 추적

| 기준 | 현재 증거 | 상태 |
|---|---|---|
| SC-001 | 시작 화면 3단계 안내와 핵심 흐름 구현 | 사용자 90%/3분 연구 필요 |
| SC-002~SC-004 | 10,000-path benchmark 3.37ms, scan/read fixture | 자동 검증 PASS |
| SC-005~SC-007 | revision/snapshot/reconciliation/path safety test | 자동 검증 PASS |
| SC-008 | deterministic JSON v1 fixture와 contract shape | 자동 검증 PASS |
| SC-009 | ARIA tree/keyboard test, 실제 `.app` AX tree와 Tab 포커스 smoke | VoiceOver 음성 및 keyboard-only 전체 흐름 확인 필요 |
| SC-010 | 실제 `.app` About/폴더·파일/CLI 설치·사용·제거/Finder/기본 앱 smoke | clean macOS 및 signing credential 필요 |
| SC-011 | AW 401 tests, shared 13 tests, app boundary | 자동 검증 PASS |
| SC-012 | 명시적 HTTPS open 외 network API 부재, redacted diagnostics | 코드 검사 PASS |
