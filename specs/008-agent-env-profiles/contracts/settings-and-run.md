# Contracts: 설정 저장/조회와 실행 요청

Tauri command 이름·시그니처는 변경 없다. payload 스키마만 하위 호환으로 확장된다(신규 필드는 전부 옵션/기본값).

## 1. `get_agent_run_settings` / `save_agent_run_settings` (payload 확장)

`AgentRunSettings.commandOverrides`:

```jsonc
{
  "globalCommand": "npx -y ...",        // 유지 (옵션)
  "agentCommands": { "codex": "..." },  // 유지 — legacy 읽기 전용, 신규 UI 미편집
  "globalEnv": { "HTTPS_PROXY": "..." },            // 신규 (기본 {})
  "profiles": [                                       // 신규 (기본 [])
    {
      "id": "claude_code",              // 기본 프로필: catalog agent id 고정
      "name": "Claude (기본)",
      "agentType": "claude_code",
      "command": null,                  // null → globalCommand → catalog 기본
      "env": { "ANTHROPIC_BASE_URL": "..." },
      "enabled": true,
      "builtIn": true
    },
    {
      "id": "8f3c...-uuid",             // 커스텀 프로필
      "name": "Claude (프록시 경유)",
      "agentType": "claude_code",
      "command": "npx -y @agentclientprotocol/claude-agent-acp",
      "env": { "HTTPS_PROXY": "http://127.0.0.1:8888" },
      "enabled": true,
      "builtIn": false
    }
  ]
}
```

**저장(normalization) 규칙**
- 프로필 name/command/env key trim. 빈 name → type 기반 기본 이름. 빈 command → null. 빈/공백 env key 항목 제거(value 빈 문자열은 허용).
- 기본 프로필 4종 누락 시 seed 추가. `builtIn=true` 프로필의 삭제(목록 부재)는 seed로 복원.
- 활성 기본 프로필 0개가 되는 payload는 **오류로 거부**: `"At least one built-in agent profile must stay enabled."` (env value는 어떤 오류 메시지에도 포함하지 않는다.)

**로드(하위 호환) 규칙**
- `globalEnv`/`profiles` 필드가 없는 기존 파일 → 기본값(빈 map/배열)으로 역직렬화 후 seed.
- seed 시 legacy `agentCommands[agentType]`을 기본 프로필 command 초기값으로 사용.
- 저장 파일의 legacy 필드는 제거하지 않는다.

## 2. `start_agent_run` (request 확장)

```jsonc
{
  "agentId": "claude_code",            // 실행 provider = 프로필의 agentType
  "agentCommand": "npx -y ...",        // 해석 완료된 command (유지)
  "agentEnv": { "FOO": "bar" },        // 신규(옵션): globalEnv ⊕ profile.env 병합 완료본
  // ... 기존 필드 불변
}
```

- `agentEnv` 부재/빈 map → 현행과 동일 동작(주입 없음).
- runner 주입 규칙: `Command::envs(agentEnv)` 적용. `PATH` key 존재 시 `agentEnv.PATH + ":" + enriched_path()`로 결합, 부재 시 기존 enriched PATH 주입 유지. 프로그램 resolve는 현행 유지.
- 오류·로그에는 env key만 표기 가능, value 금지.

## 3. Frontend 해석 계약 (`features/agent-command-override/model`)

```ts
resolveAgentProfileLaunch({ profileId, overrides, agents }): {
  agentId: string;          // = profile.agentType (세션/provider 흐름용)
  command: string;          // profile.command ?? globalCommand ?? catalog 기본
  env: Record<string, string>; // globalEnv ⊕ profile.env
  source: "profileCommand" | "globalOverride" | "defaultCommand";
} | null                    // 프로필/기본 command 부재 시
```

- `effectiveProfiles(overrides)`: seed + legacy 매핑이 적용된 프로필 목록(로드 표시용, 저장과 무관).
- `canDisableProfile(profiles, id)`: 마지막 활성 기본 프로필이면 false + 사유.

## 4. UI 계약

- Settings: 프로필 목록(기본/커스텀 구분 표시), 프로필별 name/type/command/env 편집, 커스텀 추가·삭제, 기본 프로필 삭제 버튼 없음, 마지막 활성 기본 프로필 disable 토글 비활성 + 안내 문구, global command/env 섹션.
- 세션 시작(agent-run 패널): agent 선택 목록 = `enabled` 프로필(이름 + type 병기). 선택 저장은 profile id, 부재/disabled 시 첫 enabled 프로필 폴백.
