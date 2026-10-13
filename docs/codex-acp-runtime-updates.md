# Codex ACP 런타임 업데이트와 모델 조회

AW의 기본 Codex 명령은 `npx -y @agentclientprotocol/codex-acp@latest`다. 기존 `1.1.5` 고정과 모델 스냅샷으로 신규 모델이 누락되는 문제(gh-217)를 해결한다.

## 모델 목록

기본 카탈로그는 다음 명령으로 Codex App Server를 실행하고, 초기화 후 `model/list`를 호출한다.

```sh
npx -y --prefer-online --package @agentclientprotocol/codex-acp@latest codex app-server
```

- 응답의 `model`을 실행용 ID, `displayName`을 표시 이름으로 사용한다. 숨김 모델을 제외하고 모든 페이지를 조회한다.
- 추론 수준은 반환된 `supportedReasoningEfforts`의 합집합으로 구성한다. 실제 모델별 지원 여부는 세션 생성·복원 시 ACP `configOptions`로 다시 검증한다.
- 조회는 추론 요청이나 새 대화 생성 없이 끝나며, 프로세스를 종료한다.
- 성공 결과는 프로세스 안에서 5분 캐시한다. 조회 제한 시간은 30초이고 실패 시 15초 이후 다시 조회할 수 있다. 실패를 정적 모델 목록으로 대체하지 않는다.
- 사용자 정의 카탈로그의 별도 Codex 명령과 모델 목록은 유지한다. 프로필별 환경 및 계정이 기본 조회 환경과 다르면 실행 세션의 광고값이 최종 기준이다.

## 실행 전 업데이트

`npx … codex-acp@latest` 명령으로 세션을 시작할 때 다음 준비를 수행한다.

1. `npm view @agentclientprotocol/codex-acp@latest version --json`으로 최신 버전을 확인한다(20초 제한).
2. 마지막으로 준비한 버전과 비교하여 확인 중 또는 업데이트 중 메시지를 run 타임라인에 표시한다. 첫 실행에서는 설치/업데이트 준비 메시지를 표시한다.
3. `npx --prefer-online … --version`으로 필요한 패키지를 설치한다(180초 제한). 대기 중 5초마다 진행 메시지를 표시한다.
4. 출력 버전이 조회한 최신 버전인지 검증한 후 준비 완료를 표시한다. 성공한 버전만 `~/.cache/agentic-workbench/codex-acp-version`에 기록한다.
5. 준비가 끝나면 기존 ACP 초기화와 세션 생성·복원을 수행한다. 업데이트 대기는 idle 세션의 기존 30초 초기화 제한에 포함하지 않는다.

버전 조회 실패, 설치 실패, 설치 시간 초과, 오래된 캐시의 버전 불일치는 서로 다른 오류 메시지로 안내하며 세션 시작을 중단한다. 실패를 업데이트 완료로 기록하지 않는다. 명시적 버전 고정이나 사용자 래퍼 명령에는 자동 업데이트 준비를 적용하지 않는다.

## 검증

```sh
cargo test -p acp-agent-core --lib
cargo test -p acp-agent-core discovers_real_models -- --ignored
pnpm --filter @yoophi/agentic-workbench test src/features/agent-command-override/ui/agent-command-override-editor.test.tsx
```

실제 조회 검증에는 npm 네트워크와 로컬 Codex 인증 환경이 필요하다. 모델 목록 조회와 설정 변경 성공은 유료 추론 요청이나 모든 계정의 모델 접근 권한을 보장하지 않는다.

참고: [Codex App Server](https://learn.chatgpt.com/docs/app-server), [codex-acp](https://github.com/agentclientprotocol/codex-acp), [ACP Session Config Options v1](https://agentclientprotocol.com/protocol/v1/session-config-options).

## 독립 서버와 릴리스 설치

AW의 기본 external 모드에서는 모델 목록과 실행 명령을 독립 Workbench 서버가 제공한다. 데스크톱 앱만 교체해도, 같은 데이터 디렉터리를 소유한 이전 서버가 살아 있으면 이전 카탈로그를 계속 사용한다. 따라서 앱의 About 버전이나 프로세스 기동만으로 설치를 검증할 수 없다.

릴리스 스크립트는 `agentic-workbench-server`도 같은 소스와 CALVER로 빌드하고 Tauri `externalBin`을 통해 `Contents/MacOS`에 포함한다. 앱 및 DMG 안에 서버 실행 파일이 존재하는지, arm64인지, 서명이 유효한지 검증한다. 번들 서버의 격리된 임시 데이터 디렉터리 기동과 실제 광고 버전도 확인한다.

설치 시에는 기존 서버에 `stop --data-dir <AW data dir>`를 보내 정상 종료하고 새 번들 서버로 전환한다. 기본 stop은 활동 작업이 있으면 거절하므로 강제 종료로 우회하지 않는다. 공유 서버를 쓰던 다른 클라이언트도 서버 epoch 변경 후 다시 연결한다. 설치 후에는 실제 서버의 `agent.list`에서 `codex-acp@latest` 및 새 모델 목록을 확인한다.
