import { useState } from "react";
import { PlusIcon, Trash2Icon } from "lucide-react";

import type { AgentDescriptor, AgentType } from "@/entities/agent-run/model/types";
import {
  BUILT_IN_AGENT_PROFILES,
  builtInProfileDefaultName,
} from "@/features/agent-command-override/model/command-overrides";
import type {
  CommandOverrideDraft,
  ProfileDraft,
} from "@/features/agent-command-override/model/command-override-form";
import {
  addCustomProfileDraft,
  addEnvRow,
  removeCustomProfileDraft,
  removeEnvRow,
  updateEnvRow,
  updateProfileDraft,
} from "@/features/agent-command-override/model/command-override-form";
import { canDisableProfile } from "@/features/agent-command-override/model/profile-invariants";
import { EnvVarEditor } from "@/features/agent-command-override/ui/env-var-editor";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

type AgentCommandOverrideEditorProps = {
  agents: AgentDescriptor[];
  draft: CommandOverrideDraft;
  isSaving?: boolean;
  saveError?: string | null;
  loadError?: string | null;
  onDraftChange: (draft: CommandOverrideDraft) => void;
  onSave: () => void;
};

/**
 * agent 프로필 편집기(specs/008). 기본 프로필 4종은 수정/비활성화만 가능하고,
 * 커스텀 프로필은 같은 type으로 여러 개 추가·삭제할 수 있다.
 */
export function AgentCommandOverrideEditor({
  agents,
  draft,
  isSaving = false,
  saveError = null,
  loadError = null,
  onDraftChange,
  onSave,
}: AgentCommandOverrideEditorProps) {
  const [newProfileType, setNewProfileType] = useState<AgentType>("codex");
  // draft(저장 전 payload 후보) 기준으로 disable 가능 여부를 판정한다(FR-010).
  const invariantProfiles = draft.profiles.map((profile) => ({
    id: profile.id,
    name: profile.name,
    agentType: profile.agentType,
    command: profile.command || null,
    env: {},
    enabled: profile.enabled,
    builtIn: profile.builtIn,
  }));

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h2 className="text-base font-medium">Global override</h2>
            <p className="text-sm text-muted-foreground">
              프로필에 command가 없을 때 사용할 공통 ACP 실행 명령입니다.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onDraftChange({ ...draft, globalCommand: "" })}
          >
            초기화
          </Button>
        </div>
        <Input
          value={draft.globalCommand}
          placeholder="예: npx -y @agentclientprotocol/codex-acp"
          className="font-mono"
          onChange={(event) =>
            onDraftChange({ ...draft, globalCommand: event.target.value })
          }
        />
        <div>
          <h3 className="text-sm font-medium">Global 환경변수</h3>
          <p className="text-xs text-muted-foreground">
            모든 프로필 실행에 병합됩니다. 동일 key는 프로필 값이 우선합니다.
          </p>
        </div>
        <EnvVarEditor
          rows={draft.globalEnv}
          onAddRow={() => onDraftChange(addEnvRow(draft, {}))}
          onUpdateRow={(rowId, key, value) =>
            onDraftChange(updateEnvRow(draft, { rowId, key, value }))
          }
          onRemoveRow={(rowId) => onDraftChange(removeEnvRow(draft, { rowId }))}
        />
      </section>

      <section className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h2 className="text-base font-medium">Agent 프로필</h2>
            <p className="text-sm text-muted-foreground">
              같은 agent 종류를 서로 다른 command/환경변수 조합으로 등록해 세션
              시작 시 선택할 수 있습니다.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Select
              value={newProfileType}
              onValueChange={(value) => setNewProfileType(value as AgentType)}
            >
              <SelectTrigger className="w-44" aria-label="추가할 프로필의 agent 종류">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {BUILT_IN_AGENT_PROFILES.map((entry) => (
                  <SelectItem key={entry.agentType} value={entry.agentType}>
                    {entry.defaultName}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => onDraftChange(addCustomProfileDraft(draft, newProfileType))}
            >
              <PlusIcon data-icon="inline-start" />
              프로필 추가
            </Button>
          </div>
        </div>

        <div className="grid gap-3">
          {draft.profiles.map((profile) => (
            <ProfileCard
              key={profile.id}
              profile={profile}
              agents={agents}
              disableVerdict={canDisableProfile(invariantProfiles, profile.id)}
              onChange={(patch) => onDraftChange(updateProfileDraft(draft, profile.id, patch))}
              onRemove={() => onDraftChange(removeCustomProfileDraft(draft, profile.id))}
              onAddEnvRow={() => onDraftChange(addEnvRow(draft, { profileId: profile.id }))}
              onUpdateEnvRow={(rowId, key, value) =>
                onDraftChange(updateEnvRow(draft, { profileId: profile.id, rowId, key, value }))
              }
              onRemoveEnvRow={(rowId) =>
                onDraftChange(removeEnvRow(draft, { profileId: profile.id, rowId }))
              }
            />
          ))}
        </div>
      </section>

      {(loadError || saveError) && (
        <p className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
          {loadError ?? saveError}
        </p>
      )}

      <div className="flex justify-end">
        <Button type="button" onClick={onSave} disabled={isSaving || Boolean(loadError)}>
          {isSaving ? "저장 중" : "저장"}
        </Button>
      </div>
    </div>
  );
}

function ProfileCard({
  profile,
  agents,
  disableVerdict,
  onChange,
  onRemove,
  onAddEnvRow,
  onUpdateEnvRow,
  onRemoveEnvRow,
}: {
  profile: ProfileDraft;
  agents: AgentDescriptor[];
  disableVerdict: { allowed: boolean; reason?: string };
  onChange: (
    patch: Partial<Pick<ProfileDraft, "name" | "command" | "enabled">>,
  ) => void;
  onRemove: () => void;
  onAddEnvRow: () => void;
  onUpdateEnvRow: (rowId: string, key: string, value: string) => void;
  onRemoveEnvRow: (rowId: string) => void;
}) {
  const defaultCommand = agents.find((agent) => agent.id === profile.agentType)?.command;
  const disableBlocked = profile.enabled && !disableVerdict.allowed;

  return (
    <div className="flex flex-col gap-3 rounded-lg border bg-background p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <Input
            value={profile.name}
            className="h-8 w-56 font-medium"
            aria-label="프로필 이름"
            onChange={(event) => onChange({ name: event.target.value })}
          />
          <Badge variant="outline" className="shrink-0 font-mono">
            {builtInProfileDefaultName(profile.agentType)}
          </Badge>
          <Badge variant={profile.builtIn ? "secondary" : "default"} className="shrink-0">
            {profile.builtIn ? "기본" : "커스텀"}
          </Badge>
          {!profile.enabled && (
            <Badge variant="outline" className="shrink-0 text-muted-foreground">
              비활성
            </Badge>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={disableBlocked}
            aria-label={`${profile.name} 프로필 ${profile.enabled ? "비활성화" : "활성화"}`}
            onClick={() => onChange({ enabled: !profile.enabled })}
          >
            {profile.enabled ? "비활성화" : "활성화"}
          </Button>
          {/* 기본 프로필은 삭제할 수 없다(FR-009) — 삭제 버튼 자체를 제공하지 않는다. */}
          {!profile.builtIn && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="프로필 삭제"
              onClick={onRemove}
            >
              <Trash2Icon />
            </Button>
          )}
        </div>
      </div>

      {disableBlocked && disableVerdict.reason && (
        <p className="text-xs text-destructive">{disableVerdict.reason}</p>
      )}

      <div className="flex flex-col gap-1">
        <span className="text-xs font-medium text-muted-foreground">실행 명령</span>
        <Input
          value={profile.command}
          placeholder={defaultCommand ? `기본: ${defaultCommand}` : "기본 명령 사용"}
          className="font-mono"
          onChange={(event) => onChange({ command: event.target.value })}
        />
      </div>

      <div className="flex flex-col gap-1">
        <span className="text-xs font-medium text-muted-foreground">환경변수</span>
        <EnvVarEditor
          rows={profile.env}
          onAddRow={onAddEnvRow}
          onUpdateRow={onUpdateEnvRow}
          onRemoveRow={onRemoveEnvRow}
        />
      </div>
    </div>
  );
}
