# Implementation Plan: 데스크톱 화면을 Workbench 네트워크 경로로 전환 (043, 4단계)

**Branch**: `043-frontend-http` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/043-frontend-http/spec.md`

## Summary

화면의 서버 소유 호출과 이벤트 수신을 042 네트워크 경로로 옮긴다. `packages/workbench-client`에 운영용 호출 클라이언트(세 가지 실패 구분·같은 세대 재시도)와 이벤트 클라이언트(스트림당 연결, **화면 반영 완료 cursor**, 수신자 교체 큐, 사유별 gap 복구, 세대 변경 재동기)를 만든다. 창 사이 격리는 **창별 데스크톱 주체**(`desktop:window:<label>`)로 서버의 작업대 소유 판정이 강제한다. 경로는 **창 부팅 때 한 번** 정한다 — 연결 실패면 처음부터 호환 경로, 네트워크 경로로 시작한 창은 끊겨도 재연결만 한다. 네트워크 경로 창에는 앱 내부 이벤트 전달을 끈다. compat command·전달 코드는 남긴다(8단계).

## Technical Context

**Language/Version**: TypeScript 5(React 19 화면, 공유 클라이언트), Rust 2021/2024(protocol·server·AW 조립의 작은 변경)

**Primary Dependencies**: 계약 생성 타입(`openapi-typescript`, 037), 브라우저 `fetch`·`WebSocket`(WebView 기본), Tauri `invoke`(연결 정보·창 작업대·전달 선언만). 새 런타임 의존 없음.

**Storage**: 없음(자격 증명은 메모리).

**Testing**: vitest(클라이언트 단위·저장소 동등성·기존 화면 시험), cargo test(창별 주체 격리·계약), 실제 앱 debug probe(개발·배포 출처)

**Target Platform**: macOS(개발·스모크), 배포 출처 `tauri://localhost` 실측됨(042). Windows 출처는 목록만.

**Project Type**: desktop-app + 공유 TS 패키지 + 공유 Rust 크레이트

**Performance Goals**: 호출 지연이 호환 경로 대비 체감 차이 없음(루프백), run 출력 스트림 지연 증가 없음

**Constraints**: 화면 문구·배치 불변(연결 상태 표시만 추가), `packages/agent-client`·`crates/acp-agent-core` diff 0, 한 창 한 경로, 새 세대 자동 재전송 금지

**Scale/Scope**: 저장소 21개 모듈의 서버 소유 호출 전부(호환 command 67개 중 데스크톱 표현 상태 제외), 이벤트 7종 + Worktree 알림

## Constitution Check

- **I. Monorepo Boundary First**: `apps/agentic-workbench`, `packages/workbench-client`(AW 전용), `crates/workbench-protocol`·`workbench-server`(주체 생성자·발급 인자). `packages/agent-client`·`acp-agent-core` 불변 — 통과.
- **II. FSD**: 저장소 `entities/*/api`, transport·연결 수명 `shared/api`, 부팅 `app`, 연결 상태 `widgets` — 통과.
- **III. Hexagonal Tauri**: 새 command(`ensure_window_bench`, `declare_network_delivery`)는 inbound, 전달 표는 infrastructure(`tauri_desktop_bridge`), 창별 주체는 조립 — 통과.
- **IV. Shared core vs UI**: 클라이언트는 화면 없는 순수 코드 — 통과.
- **V. Persistence and safety**: 새 영속 없음. 창 격리는 서버 판정(R1). 자격 증명 비저장·비기록 — 통과.
- **VI. Documentation**: seam 문서 화면 전환 절, 연구 각주, 연결 상태 Storybook — 통과.

설계 뒤 재검사: 같음(위반 없음).

## Project Structure

### Documentation (this feature)

```text
specs/043-frontend-http/
├── plan.md
├── research.md          # R1–R12
├── data-model.md
├── quickstart.md
├── contracts/
│   └── client-contract.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-protocol/src/principal.rs     # AuthenticatedPrincipal::desktop_window(label)
crates/workbench-server/src/auth.rs            # DesktopTokenIssuer: 토큰에 주체를 묶는 발급
crates/workbench-core/tests/window_isolation.rs # 다른 창 주체의 작업대·run·교환·orchestration 조작·구독 거절
crates/workbench-core/tests/retention_resubscribe.rs # 보관 한도 초과 gap의 lastSequence로 재구독 → live 등록
crates/workbench-core/examples/http_test_host.rs    # TS 통합 suite용 시험 host(작은 보관 한도, 고정 토큰, 운영 router)

packages/workbench-client/src/
├── call-client.ts        # createWorkbenchClient: call(), 세 결과, 같은 세대 재시도, 401 갱신
├── test/integration/     # 시험 host(실제 042 router·hub, 작은 보관 한도)에 붙는 복구 통합 suite
├── event-client.ts       # createEventClient: 스트림당 WS, 반영 완료 cursor, 수신자 큐, gap 복구 hook
├── connection.ts         # 자격 증명 수명(80% 갱신), 연결 상태, backoff
├── fault-string.ts       # faultToString (호환 층과 같은 규칙)
└── *.test.ts             # 가짜 fetch·WS 서버로 R6–R9, 강제 끊김 100회

apps/agentic-workbench/src/
├── app/bootstrap-transport.ts           # 부팅 때 경로 선택(R3), 전달 선언(R4)
├── shared/api/transport/                # Transport 인터페이스, CompatTransport, HttpTransport
├── entities/*/api/*-repository.ts       # transport 경유로 변경(입력·출력 매퍼 포함)
├── entities/*/api/*.parity.test.ts      # 두 transport 동등성
└── widgets/connection-status/           # 연결 상태 표시 + story

apps/agentic-workbench/src-tauri/src/
├── infrastructure/workbench_http.rs     # 창별 주체 토큰 발급, get_workbench_connection 응답
├── infrastructure/desktop_benches.rs    # 창 주체로 작업대 열기
├── infrastructure/tauri_desktop_bridge.rs # 네트워크 전달 창 표(전달 건너뜀)
├── inbound/workbench_compat.rs          # 호환 경로도 창 주체로 호출
├── inbound/tauri_commands.rs            # ensure_window_bench, declare_network_delivery
└── infrastructure/http_probe.rs         # probe 확장(debug): 앱 transport로 흐름·강제 끊김
```

**Structure Decision**: 네트워크 클라이언트는 `packages/workbench-client`(AW 전용 공유), 경로 선택·저장소는 AW 화면. 서버 계약은 주체 생성자·발급 인자만 바뀐다.

## Phases (tasks 입력)

1. **Setup**: 기준선 게이트 기록, 호환 command 인벤토리에서 서버 소유/데스크톱 표현 구분 표(저장소 함수 ↔ command ↔ operation).
2. **Foundational**: 창별 주체(incarnation 포함, protocol·server·AW 조립), **호환 경로 command 전부를 창 주체로**(설계 리뷰 D2, command별 주체 열 인벤토리), 창 닫힘 토큰 폐기, 격리 시험(시험 먼저 — 다른 창·같은 label 재개), `ensure_window_bench`·`declare_network_delivery`, incarnation 키 전달 표.
3. **US1(P1)**: 호출 클라이언트(R5·R6, 시험 먼저), Transport 인터페이스·두 구현, 저장소 이관(모듈별, 동등성 시험), 부팅 경로 선택(R3).
4. **US2(P1)**: 이벤트 클라이언트(R7 — Promise settle·수신자별 cursor·부분 실패 재동기, 시험 먼저), run·교환·제목·orchestration·Worktree 구독 이관, 교환 원장·재조정과 `exchange-delivery:<requestId>` 키, 전달 끄기 확인(SC-003).
5. **US3(P2)**: 재연결·gap 복구(R8 — gap `lastSequence`로 live 먼저·버퍼·스냅샷 병합)·세대 재동기(R9, 강제 끊김 100회), **실제 042 hub 보관 한도 초과 시험**(Rust + 시험 host에 붙는 TS 통합 suite), 응답 유실 재시도 규칙(SC-004b), 연결 상태 표시(R10).
6. **US4(P3)**: 창·작업대 수명(두 세션 창 격리·창 닫기), 서버 판정 시험 재확인.
7. **US5(P3)**: 부팅 실패 시 호환 경로(끝점 기동 실패 주입), 자동 전환 없음 시험.
8. **Polish**: probe 확장·실제 앱 스모크(개발·배포 출처, 강제 끊김·재연결), docs·ADR, 게이트, SC 증거, PR.

## Complexity Tracking

없음.
