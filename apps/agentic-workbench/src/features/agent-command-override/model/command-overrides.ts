import type {
  AgentCommandOverrides,
  AgentCommandSource,
  AgentDescriptor,
  AgentProfile,
  AgentType,
  CommandResolutionResult,
} from "@/entities/agent-run/model/types";

export const APP_COMMAND_OVERRIDE_SETTINGS_KEY = "__app_agent_command_overrides__";

/**
 * 기본 프로필 4종(specs/008). id/agentType은 실제 agent catalog id와 동일해
 * legacy agentCommands 매핑·세션 재사용(agent id 기반)과 무변경으로 호환된다.
 */
export const BUILT_IN_AGENT_PROFILES: ReadonlyArray<{
  agentType: AgentType;
  defaultName: string;
}> = [
  { agentType: "codex", defaultName: "Codex" },
  { agentType: "claude-code", defaultName: "Claude Code" },
  { agentType: "opencode", defaultName: "OpenCode" },
  { agentType: "pi-coding-agent", defaultName: "Pi Coding Agent" },
];

export function builtInProfileDefaultName(agentType: string) {
  return (
    BUILT_IN_AGENT_PROFILES.find((entry) => entry.agentType === agentType)?.defaultName ??
    agentType
  );
}

/** env normalization(FR-004): key trim, 빈/공백 key 제거, 빈 value는 유지. */
export function normalizeEnv(
  env: Record<string, string> | null | undefined,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(env ?? {})
      .map(([key, value]) => [key.trim(), value] as const)
      .filter(([key]) => key.length > 0),
  );
}

function normalizeProfile(profile: AgentProfile): AgentProfile {
  const name = profile.name.trim();
  const command = profile.command?.trim() || null;

  return {
    ...profile,
    id: profile.id.trim(),
    name: name.length > 0 ? name : builtInProfileDefaultName(profile.agentType),
    command,
    env: normalizeEnv(profile.env),
  };
}

/**
 * 저장 프로필 + seed(누락 기본 프로필 자동 채움)를 합친 "유효 프로필" 목록.
 * seed 시 legacy `agentCommands[agentType]`을 기본 프로필 command 초기값으로
 * 사용한다. 저장 데이터는 변경하지 않는다(specs/008 research R2).
 */
export function effectiveProfiles(
  overrides: AgentCommandOverrides | null | undefined,
): AgentProfile[] {
  const normalized = normalizeCommandOverrides(overrides);
  const stored = normalized.profiles ?? [];
  const storedIds = new Set(stored.map((profile) => profile.id));
  const seeded = BUILT_IN_AGENT_PROFILES.filter(
    (entry) => !storedIds.has(entry.agentType),
  ).map((entry) => ({
    id: entry.agentType,
    name: entry.defaultName,
    agentType: entry.agentType,
    command: normalized.agentCommands?.[entry.agentType] ?? null,
    env: {},
    enabled: true,
    builtIn: true,
  }));

  return [...stored, ...seeded];
}

export function normalizeCommandOverrides(
  overrides: AgentCommandOverrides | null | undefined,
): AgentCommandOverrides {
  const globalCommand = normalizeOptionalCommand(overrides?.globalCommand ?? null);
  const agentCommands = Object.fromEntries(
    Object.entries(overrides?.agentCommands ?? {})
      .map(([agentId, command]) => [agentId.trim(), command.trim()] as const)
      .filter(([agentId, command]) => agentId.length > 0 && command.length > 0),
  );
  const globalEnv = normalizeEnv(overrides?.globalEnv);
  const profiles = (overrides?.profiles ?? [])
    .map(normalizeProfile)
    .filter((profile) => profile.id.length > 0);

  return {
    ...(globalCommand ? { globalCommand } : {}),
    ...(Object.keys(agentCommands).length > 0 ? { agentCommands } : {}),
    ...(Object.keys(globalEnv).length > 0 ? { globalEnv } : {}),
    ...(profiles.length > 0 ? { profiles } : {}),
  };
}

export function resolveAgentCommand({
  agentId,
  agents,
  overrides,
}: {
  agentId: string;
  agents: AgentDescriptor[];
  overrides: AgentCommandOverrides | null | undefined;
}): CommandResolutionResult | null {
  const normalizedAgentId = agentId.trim();
  const normalizedOverrides = normalizeCommandOverrides(overrides);
  const agentCommand = normalizedOverrides.agentCommands?.[normalizedAgentId];
  if (agentCommand) {
    return {
      agentId: normalizedAgentId,
      command: agentCommand,
      source: "agentOverride",
    };
  }

  if (normalizedOverrides.globalCommand) {
    return {
      agentId: normalizedAgentId,
      command: normalizedOverrides.globalCommand,
      source: "globalOverride",
    };
  }

  const defaultCommand = agents.find((agent) => agent.id === normalizedAgentId)?.command.trim();
  if (!defaultCommand) {
    return null;
  }

  return {
    agentId: normalizedAgentId,
    command: defaultCommand,
    source: "defaultCommand",
  };
}

export type AgentProfileLaunch = {
  /** 실행 provider = 프로필의 agentType(세션 재사용 등 agent id 기반 흐름용). */
  agentId: string;
  command: string;
  /** globalEnv ⊕ profile.env — 동일 key는 프로필 값 우선(specs/008 R4). */
  env: Record<string, string>;
  source: AgentCommandSource;
};

/**
 * 선택된 프로필의 실행 구성(command + env)을 해석한다(contracts §3).
 * command 폴백: profile.command → globalCommand → catalog 기본.
 */
export function resolveAgentProfileLaunch({
  profileId,
  overrides,
  agents,
}: {
  profileId: string;
  overrides: AgentCommandOverrides | null | undefined;
  agents: AgentDescriptor[];
}): AgentProfileLaunch | null {
  const normalized = normalizeCommandOverrides(overrides);
  const profile = effectiveProfiles(normalized).find((entry) => entry.id === profileId.trim());
  if (!profile) {
    return null;
  }

  const env = { ...normalizeEnv(normalized.globalEnv), ...normalizeEnv(profile.env) };

  if (profile.command) {
    return { agentId: profile.agentType, command: profile.command, env, source: "profileCommand" };
  }

  if (normalized.globalCommand) {
    return {
      agentId: profile.agentType,
      command: normalized.globalCommand,
      env,
      source: "globalOverride",
    };
  }

  const defaultCommand = agents
    .find((agent) => agent.id === profile.agentType)
    ?.command.trim();
  if (!defaultCommand) {
    return null;
  }

  return { agentId: profile.agentType, command: defaultCommand, env, source: "defaultCommand" };
}

export function sourceLabel(source: AgentCommandSource) {
  switch (source) {
    case "agentOverride":
      return "Agent override";
    case "globalOverride":
      return "Global override";
    case "defaultCommand":
      return "Default command";
  }
}

function normalizeOptionalCommand(value: string | null | undefined) {
  const trimmed = value?.trim() ?? "";
  return trimmed.length > 0 ? trimmed : undefined;
}
