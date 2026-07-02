import { describe, expect, it } from "vitest";

import type { AgentProfile } from "@/entities/agent-run/model/types";

import { canDisableProfile, hasActiveBuiltInProfile } from "./profile-invariants";

function profile(overrides: Partial<AgentProfile>): AgentProfile {
  return {
    id: "codex",
    name: "Codex",
    agentType: "codex",
    command: null,
    env: {},
    enabled: true,
    builtIn: true,
    ...overrides,
  };
}

describe("profile invariants (specs/008 FR-010)", () => {
  it("blocks disabling the last active built-in profile with a reason", () => {
    const profiles = [
      profile({ id: "codex", enabled: true }),
      profile({ id: "claude-code", agentType: "claude-code", enabled: false }),
      profile({ id: "custom-1", builtIn: false, enabled: true }),
    ];

    const verdict = canDisableProfile(profiles, "codex");

    expect(verdict.allowed).toBe(false);
    expect(verdict.reason).toBeTruthy();
  });

  it("allows disabling a built-in profile when another built-in stays active", () => {
    const profiles = [
      profile({ id: "codex", enabled: true }),
      profile({ id: "claude-code", agentType: "claude-code", enabled: true }),
    ];

    expect(canDisableProfile(profiles, "codex").allowed).toBe(true);
  });

  it("always allows disabling custom profiles", () => {
    const profiles = [
      profile({ id: "codex", enabled: false }),
      profile({ id: "claude-code", agentType: "claude-code", enabled: true }),
      profile({ id: "custom-1", builtIn: false, enabled: true }),
    ];

    expect(canDisableProfile(profiles, "custom-1").allowed).toBe(true);
  });

  it("detects payloads with zero active built-in profiles", () => {
    expect(hasActiveBuiltInProfile([profile({ enabled: false })])).toBe(false);
    expect(hasActiveBuiltInProfile([profile({ enabled: true })])).toBe(true);
    expect(
      hasActiveBuiltInProfile([profile({ builtIn: false, enabled: true })]),
    ).toBe(false);
  });
});
