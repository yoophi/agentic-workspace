import { describe, expect, it } from "vitest";

import type { AgentCommandOverrides } from "@/entities/agent-run/model/types";
import {
  addCustomProfileDraft,
  addEnvRow,
  commandOverridePayload,
  createCommandOverrideDraft,
  removeCustomProfileDraft,
  removeEnvRow,
  updateEnvRow,
  updateProfileDraft,
} from "./command-override-form";

// 프로필 기반 폼 상태(specs/008). draft는 effectiveProfiles(seed 포함)에서 만들어지고
// 저장 payload는 normalization을 거친다.

describe("createCommandOverrideDraft (specs/008 profiles)", () => {
  it("creates profile drafts from effective profiles including seeds", () => {
    const draft = createCommandOverrideDraft({
      globalCommand: "global-acp",
      agentCommands: { codex: "legacy-codex" },
      globalEnv: { SHARED: "1" },
    });

    expect(draft.globalCommand).toBe("global-acp");
    expect(draft.globalEnv).toEqual([expect.objectContaining({ key: "SHARED", value: "1" })]);
    expect(draft.profiles).toHaveLength(4);
    const codex = draft.profiles.find((profile) => profile.id === "codex");
    expect(codex?.command).toBe("legacy-codex");
    expect(codex?.builtIn).toBe(true);
  });
});

describe("profile draft CRUD", () => {
  const baseDraft = createCommandOverrideDraft({});

  it("adds custom profiles with unique ids and builtIn=false", () => {
    const draft = addCustomProfileDraft(baseDraft, "claude-code");

    expect(draft.profiles).toHaveLength(5);
    const custom = draft.profiles[draft.profiles.length - 1];
    expect(custom.builtIn).toBe(false);
    expect(custom.agentType).toBe("claude-code");
    expect(custom.id).not.toBe("claude-code");
    expect(custom.enabled).toBe(true);

    const again = addCustomProfileDraft(draft, "claude-code");
    const ids = again.profiles.map((profile) => profile.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("removes only custom profiles", () => {
    const withCustom = addCustomProfileDraft(baseDraft, "codex");
    const customId = withCustom.profiles[withCustom.profiles.length - 1].id;

    const removed = removeCustomProfileDraft(withCustom, customId);
    expect(removed.profiles).toHaveLength(4);

    // 기본 프로필 삭제 시도는 무시된다.
    expect(removeCustomProfileDraft(removed, "codex").profiles).toHaveLength(4);
  });

  it("updates profile fields immutably", () => {
    const updated = updateProfileDraft(baseDraft, "codex", {
      name: "Codex 프록시",
      command: "npx custom",
      enabled: false,
    });

    const codex = updated.profiles.find((profile) => profile.id === "codex");
    expect(codex?.name).toBe("Codex 프록시");
    expect(codex?.command).toBe("npx custom");
    expect(codex?.enabled).toBe(false);
    expect(baseDraft.profiles.find((profile) => profile.id === "codex")?.name).not.toBe(
      "Codex 프록시",
    );
  });
});

describe("env row editing", () => {
  const baseDraft = createCommandOverrideDraft({});

  it("adds, updates, and removes env rows on a profile", () => {
    let draft = addEnvRow(baseDraft, { profileId: "codex" });
    const rowId = draft.profiles.find((profile) => profile.id === "codex")!.env[0].id;

    draft = updateEnvRow(draft, { profileId: "codex", rowId, key: " FOO ", value: "bar" });
    expect(draft.profiles.find((profile) => profile.id === "codex")?.env[0]).toMatchObject({
      key: " FOO ",
      value: "bar",
    });

    draft = removeEnvRow(draft, { profileId: "codex", rowId });
    expect(draft.profiles.find((profile) => profile.id === "codex")?.env).toHaveLength(0);
  });

  it("edits global env rows when profileId is omitted", () => {
    let draft = addEnvRow(baseDraft, {});
    const rowId = draft.globalEnv[0].id;

    draft = updateEnvRow(draft, { rowId, key: "SHARED", value: "1" });
    expect(draft.globalEnv[0]).toMatchObject({ key: "SHARED", value: "1" });
  });
});

describe("commandOverridePayload (specs/008)", () => {
  it("normalizes env rows and keeps legacy fields for rollback compatibility", () => {
    const saved: AgentCommandOverrides = {
      agentCommands: { codex: "legacy-codex" },
    };
    let draft = createCommandOverrideDraft(saved);
    draft = addEnvRow(draft, { profileId: "codex" });
    const rowId = draft.profiles.find((profile) => profile.id === "codex")!.env[0].id;
    draft = updateEnvRow(draft, { profileId: "codex", rowId, key: "  FOO  ", value: "bar" });
    draft = addEnvRow(draft, { profileId: "codex" });

    const payload = commandOverridePayload(draft, saved);

    const codex = payload.profiles?.find((profile) => profile.id === "codex");
    expect(codex?.env).toEqual({ FOO: "bar" });
    // 빈 key 행은 저장되지 않는다(FR-004).
    expect(Object.keys(codex?.env ?? {})).toHaveLength(1);
    // legacy 필드 유지(구버전 롤백 호환, R2).
    expect(payload.agentCommands).toEqual({ codex: "legacy-codex" });
  });
});
