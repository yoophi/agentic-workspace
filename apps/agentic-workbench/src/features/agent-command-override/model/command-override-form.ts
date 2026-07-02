import type {
  AgentCommandOverrides,
  AgentProfile,
  AgentType,
} from "@/entities/agent-run/model/types";
import {
  builtInProfileDefaultName,
  effectiveProfiles,
  normalizeCommandOverrides,
  normalizeEnv,
} from "./command-overrides";

// 프로필 기반 설정 폼 상태(specs/008). env는 편집 중 순서·중간 상태(빈 key 등)를
// 보존해야 하므로 row 목록으로 다루고, 저장 payload에서 normalization한다.

export type EnvDraftRow = {
  id: string;
  key: string;
  value: string;
};

export type ProfileDraft = {
  id: string;
  name: string;
  agentType: AgentType;
  command: string;
  env: EnvDraftRow[];
  enabled: boolean;
  builtIn: boolean;
};

export type CommandOverrideDraft = {
  globalCommand: string;
  globalEnv: EnvDraftRow[];
  profiles: ProfileDraft[];
};

function envRowsFromRecord(env: Record<string, string> | undefined): EnvDraftRow[] {
  return Object.entries(env ?? {}).map(([key, value]) => ({
    id: crypto.randomUUID(),
    key,
    value,
  }));
}

function envRecordFromRows(rows: EnvDraftRow[]): Record<string, string> {
  return normalizeEnv(
    Object.fromEntries(rows.map((row) => [row.key, row.value] as const)),
  );
}

export function createCommandOverrideDraft(
  overrides: AgentCommandOverrides | null | undefined,
): CommandOverrideDraft {
  const normalized = normalizeCommandOverrides(overrides);

  return {
    globalCommand: normalized.globalCommand ?? "",
    globalEnv: envRowsFromRecord(normalized.globalEnv),
    profiles: effectiveProfiles(normalized).map((profile) => ({
      id: profile.id,
      name: profile.name,
      agentType: profile.agentType,
      command: profile.command ?? "",
      env: envRowsFromRecord(profile.env),
      enabled: profile.enabled,
      builtIn: profile.builtIn,
    })),
  };
}

export function addCustomProfileDraft(
  draft: CommandOverrideDraft,
  agentType: AgentType,
): CommandOverrideDraft {
  const sameTypeCount = draft.profiles.filter(
    (profile) => profile.agentType === agentType,
  ).length;

  return {
    ...draft,
    profiles: [
      ...draft.profiles,
      {
        id: crypto.randomUUID(),
        name: `${builtInProfileDefaultName(agentType)} ${sameTypeCount + 1}`,
        agentType,
        command: "",
        env: [],
        enabled: true,
        builtIn: false,
      },
    ],
  };
}

/** 커스텀 프로필만 삭제한다. 기본 프로필 id는 무시된다(FR-009). */
export function removeCustomProfileDraft(
  draft: CommandOverrideDraft,
  profileId: string,
): CommandOverrideDraft {
  return {
    ...draft,
    profiles: draft.profiles.filter(
      (profile) => profile.builtIn || profile.id !== profileId,
    ),
  };
}

export function updateProfileDraft(
  draft: CommandOverrideDraft,
  profileId: string,
  patch: Partial<Pick<ProfileDraft, "name" | "agentType" | "command" | "enabled">>,
): CommandOverrideDraft {
  return {
    ...draft,
    profiles: draft.profiles.map((profile) =>
      profile.id === profileId ? { ...profile, ...patch } : profile,
    ),
  };
}

type EnvRowTarget = {
  /** 생략하면 global env를 편집한다. */
  profileId?: string;
};

function mapEnvRows(
  draft: CommandOverrideDraft,
  target: EnvRowTarget,
  map: (rows: EnvDraftRow[]) => EnvDraftRow[],
): CommandOverrideDraft {
  if (!target.profileId) {
    return { ...draft, globalEnv: map(draft.globalEnv) };
  }

  return {
    ...draft,
    profiles: draft.profiles.map((profile) =>
      profile.id === target.profileId ? { ...profile, env: map(profile.env) } : profile,
    ),
  };
}

export function addEnvRow(
  draft: CommandOverrideDraft,
  target: EnvRowTarget,
): CommandOverrideDraft {
  return mapEnvRows(draft, target, (rows) => [
    ...rows,
    { id: crypto.randomUUID(), key: "", value: "" },
  ]);
}

export function updateEnvRow(
  draft: CommandOverrideDraft,
  { rowId, key, value, ...target }: EnvRowTarget & { rowId: string; key: string; value: string },
): CommandOverrideDraft {
  return mapEnvRows(draft, target, (rows) =>
    rows.map((row) => (row.id === rowId ? { ...row, key, value } : row)),
  );
}

export function removeEnvRow(
  draft: CommandOverrideDraft,
  { rowId, ...target }: EnvRowTarget & { rowId: string },
): CommandOverrideDraft {
  return mapEnvRows(draft, target, (rows) => rows.filter((row) => row.id !== rowId));
}

/** draft의 프로필을 저장용 AgentProfile로 변환한다(빈 command → null). */
function profilePayload(profile: ProfileDraft): AgentProfile {
  return {
    id: profile.id,
    name: profile.name,
    agentType: profile.agentType,
    command: profile.command.trim() || null,
    env: envRecordFromRows(profile.env),
    enabled: profile.enabled,
    builtIn: profile.builtIn,
  };
}

/**
 * 저장 payload. legacy 필드(agentCommands)는 편집하지 않고 저장본에서 그대로
 * 이어받아 구버전 롤백 호환을 지킨다(specs/008 research R2).
 */
export function commandOverridePayload(
  draft: CommandOverrideDraft,
  savedOverrides: AgentCommandOverrides | null | undefined,
): AgentCommandOverrides {
  return normalizeCommandOverrides({
    globalCommand: draft.globalCommand,
    agentCommands: savedOverrides?.agentCommands,
    globalEnv: envRecordFromRows(draft.globalEnv),
    profiles: draft.profiles.map(profilePayload),
  });
}

export function preserveDraftOnSaveError(
  draft: CommandOverrideDraft,
): CommandOverrideDraft {
  return {
    globalCommand: draft.globalCommand,
    globalEnv: draft.globalEnv.map((row) => ({ ...row })),
    profiles: draft.profiles.map((profile) => ({
      ...profile,
      env: profile.env.map((row) => ({ ...row })),
    })),
  };
}
