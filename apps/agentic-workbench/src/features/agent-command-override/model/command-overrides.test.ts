import { describe, expect, it } from "vitest";

import type {
  AgentCommandOverrides,
  AgentDescriptor,
} from "@/entities/agent-run/model/types";
import {
  effectiveProfiles,
  normalizeCommandOverrides,
  resolveAgentCommand,
} from "./command-overrides";

const agents: AgentDescriptor[] = [
  { id: "codex", label: "Codex", command: "codex-default" },
  { id: "claude-code", label: "Claude Code", command: "claude-default" },
];

describe("command override helpers", () => {
  it("normalizes blank commands and trims preserved commands", () => {
    expect(
      normalizeCommandOverrides({
        globalCommand: "  custom-acp  ",
        agentCommands: {
          " codex ": "  codex-acp  ",
          "claude-code": "   ",
          " ": "ignored",
        },
      }),
    ).toEqual({
      globalCommand: "custom-acp",
      agentCommands: {
        codex: "codex-acp",
      },
    });
  });

  it("resolves agent override before global override and default command", () => {
    expect(
      resolveAgentCommand({
        agentId: "codex",
        agents,
        overrides: {
          globalCommand: "global-acp",
          agentCommands: { codex: "codex-acp" },
        },
      }),
    ).toEqual({
      agentId: "codex",
      command: "codex-acp",
      source: "agentOverride",
    });
  });

  it("falls back from global override to default command", () => {
    expect(
      resolveAgentCommand({
        agentId: "claude-code",
        agents,
        overrides: { globalCommand: "global-acp" },
      })?.source,
    ).toBe("globalOverride");

    expect(
      resolveAgentCommand({
        agentId: "claude-code",
        agents,
        overrides: {},
      }),
    ).toEqual({
      agentId: "claude-code",
      command: "claude-default",
      source: "defaultCommand",
    });
  });
});

describe("normalizeCommandOverrides — env/프로필 (specs/008)", () => {
  it("removes env entries with blank keys and trims keys, keeping empty values", () => {
    const normalized = normalizeCommandOverrides({
      globalEnv: { "  FOO  ": "bar", "": "drop", "   ": "drop", EMPTY: "" },
      profiles: [
        {
          id: "codex",
          name: "Codex",
          agentType: "codex",
          env: { " KEY ": "v", "  ": "x" },
          enabled: true,
          builtIn: true,
        },
      ],
    });

    expect(normalized.globalEnv).toEqual({ FOO: "bar", EMPTY: "" });
    expect(normalized.profiles?.[0].env).toEqual({ KEY: "v" });
  });

  it("drops profiles-less shape unchanged (legacy round-trip)", () => {
    const normalized = normalizeCommandOverrides({
      globalCommand: " npx run ",
      agentCommands: { codex: " cmd " },
    });

    expect(normalized.globalCommand).toBe("npx run");
    expect(normalized.agentCommands).toEqual({ codex: "cmd" });
    expect(normalized.profiles).toBeUndefined();
    expect(normalized.globalEnv).toBeUndefined();
  });
});

describe("effectiveProfiles (specs/008 seed + legacy 매핑)", () => {
  it("seeds the four built-in profiles when none are stored", () => {
    const profiles = effectiveProfiles({});

    expect(profiles.map((profile) => profile.id)).toEqual([
      "codex",
      "claude-code",
      "opencode",
      "pi-coding-agent",
    ]);
    expect(profiles.every((profile) => profile.builtIn && profile.enabled)).toBe(true);
  });

  it("maps legacy agentCommands into seeded built-in profile commands", () => {
    const profiles = effectiveProfiles({
      agentCommands: { "claude-code": "npx custom-claude" },
    });

    const claude = profiles.find((profile) => profile.id === "claude-code");
    expect(claude?.command).toBe("npx custom-claude");
  });

  it("keeps stored profiles and fills only missing built-ins", () => {
    const stored: AgentCommandOverrides = {
      profiles: [
        {
          id: "claude-code",
          name: "Claude 수정본",
          agentType: "claude-code" as const,
          command: "npx modified",
          env: {},
          enabled: false,
          builtIn: true,
        },
        {
          id: "custom-1",
          name: "Codex 프록시",
          agentType: "codex" as const,
          command: null,
          env: { HTTPS_PROXY: "http://localhost:8888" },
          enabled: true,
          builtIn: false,
        },
      ],
    };

    const profiles = effectiveProfiles(stored);

    expect(profiles.find((profile) => profile.id === "claude-code")?.name).toBe("Claude 수정본");
    expect(profiles.find((profile) => profile.id === "custom-1")).toBeDefined();
    expect(profiles.filter((profile) => profile.builtIn)).toHaveLength(4);
  });
});
