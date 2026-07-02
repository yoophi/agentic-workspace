import type { AgentProfile } from "@/entities/agent-run/model/types";

// 기본 프로필 안전장치(specs/008 FR-010): 사용자가 실행 수단을 전부 잃지 않도록
// 활성 기본 프로필이 최소 1개 유지되어야 한다.

export const LAST_ACTIVE_BUILT_IN_REASON =
  "마지막으로 활성화된 기본 프로필은 비활성화할 수 없습니다. 다른 기본 프로필을 먼저 활성화하세요.";

export function hasActiveBuiltInProfile(profiles: AgentProfile[]) {
  return profiles.some((profile) => profile.builtIn && profile.enabled);
}

export function canDisableProfile(
  profiles: AgentProfile[],
  profileId: string,
): { allowed: boolean; reason?: string } {
  const target = profiles.find((profile) => profile.id === profileId);

  if (!target || !target.builtIn) {
    return { allowed: true };
  }

  const remaining = profiles.filter(
    (profile) => profile.builtIn && profile.enabled && profile.id !== profileId,
  );

  if (remaining.length === 0) {
    return { allowed: false, reason: LAST_ACTIVE_BUILT_IN_REASON };
  }

  return { allowed: true };
}
