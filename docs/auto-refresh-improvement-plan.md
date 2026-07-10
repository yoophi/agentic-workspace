# Auto-refresh 기능 개선 계획

## Context

최근 PR #98(`39fce05`)에서 파일 변경 감지 + refetch 기반 auto refresh가 세 앱(agentic-workbench / git-explorer / markdown-annotator)에 추가되었다. 조사 결과 골격은 견고하나(공유 `@yoophi/workspace-auto-refresh` 패키지, scope 한정 invalidate, staleness 판정, 30초 폴링 fallback), 다음과 같은 실질 이슈가 확인되었다.

- **[P1-A] markdown-annotator 백엔드에 창 파괴 시 watcher 정리 훅이 없다** → watcher 스레드/FD 누수, 죽은 창에 `emit_to` 반복 실패. 스펙(`specs/001-worktree-auto-refresh/research.md:69`)이 명시적으로 요구하는데 미충족. agentic-workbench는 처리됨.
- **[P1-B] markdown-annotator `reloadActiveDocument`의 stale closure** → watcher/폴링 콜백이 마운트 시점 `document.markdownText`를 고정 캡처. 파일이 이전에 표시했던 값으로 되돌아가면(`C→A`) 갱신 누락, 동일 내용 재저장 시 불필요한 재파싱.
- **[P2-C] markdown watcher가 정확 경로 일치만 검사** → 에디터의 atomic-save(임시파일→rename) 시 감지를 놓칠 수 있음(30초 폴링만 백업).
- **[P2-D] markdown-annotator 자체 `setInterval` 폴링이 백그라운드에서도 동작** → 공유 옵션의 `refetchIntervalInBackground:false` 취지 미반영.
- **[P3] 후속 권장**: 세 watcher의 leading-edge 디바운스는 burst 마지막 상태를 놓칠 여지가 있음(스펙상 의도된 설계이며 폴링이 보정). 별도 작업으로 분리.

목표: 확정 버그(P1)를 안전하게 제거하고, 저비용 견고성 개선(P2)을 반영하며, 회귀를 막을 최소 테스트를 추가한다. 동작을 크게 바꾸는 디바운스 재설계(P3)는 이번 범위에서 제외하고 문서로 남긴다.

## 변경 사항

### P1-A. markdown-annotator 창 파괴 시 watcher 정리 (필수)

**파일**: `apps/markdown-annotator/src-tauri/src/lib.rs`

- `use tauri::Manager;`(state 접근용)와 `WindowEvent`를 임포트.
- 빌더 체인에 `.on_window_event(...)`를 `.build()` 전에 추가. agentic-workbench(`apps/agentic-workbench/src-tauri/src/lib.rs:37-51`)와 동일 패턴:
  ```rust
  .on_window_event(|window, event| {
      if let WindowEvent::Destroyed = event {
          let state = window.state::<DocumentWatcherState>();
          let _ = state.stop_for_window(window.label());
      }
  })
  ```
- `DocumentWatcherState::stop_for_window`(`tauri_commands.rs:35`)는 이미 존재하며 `handles.remove(label)`이 없는 키에 no-op이므로 모든 창(`main`, `document-*`)에 안전. label 필터는 불필요.
- 프론트 `useEffect` cleanup(`AnnotatorPage.tsx:495-501`)은 그대로 두어 정상 언마운트 경로도 유지(이중 안전망).

### P1-B. reloadActiveDocument stale closure 제거 (필수)

**파일**: `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`

- 최신 document를 추적하는 `useRef`를 도입해 클로저 고정 문제를 제거한다.
  - `const latestDocumentRef = useRef(document);` 후 렌더마다 `latestDocumentRef.current = document;` 갱신(또는 `setDocument` 지점마다 동기 갱신).
  - `reloadActiveDocument` 내부의 비교/참조(`document.absolutePath`, `document.markdownText`, 298행)를 `latestDocumentRef.current` 기준으로 변경.
- 이렇게 하면 watcher/폴링 effect를 재구독하지 않고도 항상 최신 내용과 비교하게 되어 `A→B→A` 되돌림과 재저장을 정확히 감지한다.
- 대안(함수형 `setDocument(prev => ...)` 비교)은 `setPromptFilePath` 등 부수 상태 갱신과 얽혀 복잡하므로 ref 방식을 채택.

### P2-C. markdown watcher atomic-save(rename) 감지 보강

**파일**: `apps/markdown-annotator/src-tauri/src/infrastructure/fs_document_watcher.rs`

- 현재 조건(`event.paths`에 문서 경로 정확 일치, `48-54`행)에 더해, 이벤트 경로 중 **파일명이 문서 파일명과 동일한 항목**이 있으면 emit하도록 완화. 부모 디렉터리를 NonRecursive로 감시하는 현 구조를 유지하면서 rename 대상 경로가 문서 파일명과 일치하는 케이스를 포착한다.
- 판정 후 `should_emit_event` 디바운스는 그대로 통과. 오탐 위험은 같은 이름의 형제 파일 정도로 제한적이며, reload는 멱등하므로 허용 가능.

### P2-D. markdown-annotator 백그라운드 폴링 정리

**파일**: `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx` (`504-520` 폴링 effect)

- `setInterval` 콜백에서 `globalThis.document.hidden`이 true면 reload를 건너뛰도록 gate. 공유 옵션 `refetchIntervalInBackground:false`의 취지를 자체 폴링에도 반영.
- watcher가 활성일 때 폴링은 fallback이므로 이 gate는 안전.

### 테스트 (핵심만)

- `fs_document_watcher.rs`: 파일명 일치 기반 emit 판정을 순수 함수로 분리해 rename 케이스 단위 테스트 추가(기존 `#[cfg(test)]` 관례 따름 — `fs_worktree_watcher.rs:197-225` 참고).
- stale closure는 UI 통합이라 직접 테스트가 어렵다. 비교 로직을 순수 함수(예: `shouldSwapDocument(prev, next)`)로 추출해 되돌림 케이스를 vitest로 검증하고 `AnnotatorPage`가 이를 사용하도록 한다.
- 기존 테스트(`annotator-auto-reload.test.tsx`, `workspace-auto-refresh.test.ts`, watcher Rust 테스트)는 그대로 통과해야 한다.

## 범위에서 제외 (후속 권장)

- **P3 디바운스 trailing 전환**: leading-edge → trailing/coalescing은 타이머 스레드 또는 `notify-debouncer` 도입이 필요한 동작 변경이며, 스펙이 leading을 의도로 문서화했고 폴링이 보정한다. 별도 이슈로 분리.
- **agentic-workbench가 `node_modules`/`target`까지 recursive watch 등록**: macOS(FSEvents) 무영향, Linux(inotify) 대형 monorepo에서만 유의. 필요 시 별도 최적화.

## 검증

1. 빌드/타입/린트:
   - `pnpm --filter markdown-annotator build` (프론트 타입 확인)
   - `cargo build`(해당 앱) 또는 워크스페이스 `cargo check`로 Rust 컴파일 확인
2. 단위 테스트:
   - `pnpm --filter markdown-annotator test` (vitest)
   - `pnpm --filter @yoophi/workspace-auto-refresh test`
   - `cargo test`(markdown-annotator src-tauri)로 watcher 테스트
3. 수동 검증(가장 확실):
   - 문서 창을 여러 개 열고 닫은 뒤, 백엔드 로그에 죽은 창 `emit_to` 실패가 반복되지 않고 watcher 스레드가 정리되는지 확인(P1-A).
   - 열린 문서 파일을 외부에서 `A→B→A`로 되돌려 저장하고 매 단계 화면이 갱신되는지 확인(P1-B).
   - Vim 등 atomic-save 에디터로 저장 시 즉시 반영되는지 확인(P2-C).
   - 창을 백그라운드로 둔 상태에서 30초 폴링이 파일을 읽지 않는지 확인(P2-D).
