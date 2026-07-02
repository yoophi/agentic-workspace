import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  getAgentRunSettings,
  listAgents,
  saveAgentRunSettings,
} from "@/entities/agent-run/api/agent-run-repository";
import { agentRunQueryKeys } from "@/entities/agent-run/api/query-keys";
import type { AgentRunSettings } from "@/entities/agent-run/model/types";
import {
  APP_COMMAND_OVERRIDE_SETTINGS_KEY,
  normalizeCommandOverrides,
} from "@/features/agent-command-override/model/command-overrides";
import {
  commandOverridePayload,
  createCommandOverrideDraft,
  type CommandOverrideDraft,
} from "@/features/agent-command-override/model/command-override-form";
import { AgentCommandOverrideEditor } from "@/features/agent-command-override/ui/agent-command-override-editor";
import { Button } from "@/components/ui/button";

type SettingsPageProps = {
  onBack?: () => void;
};

export function SettingsPage({ onBack }: SettingsPageProps) {
  const queryClient = useQueryClient();
  const agentsQuery = useQuery({
    queryKey: agentRunQueryKeys.agents,
    queryFn: listAgents,
  });
  const settingsQueryKey = agentRunQueryKeys.settings(APP_COMMAND_OVERRIDE_SETTINGS_KEY);
  const settingsQuery = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => getAgentRunSettings(APP_COMMAND_OVERRIDE_SETTINGS_KEY),
  });
  const agents = agentsQuery.data ?? [];
  const savedOverrides = useMemo(
    () => normalizeCommandOverrides(settingsQuery.data?.commandOverrides),
    [settingsQuery.data?.commandOverrides],
  );
  const [draft, setDraft] = useState<CommandOverrideDraft>({
    globalCommand: "",
    globalEnv: [],
    profiles: [],
  });

  useEffect(() => {
    setDraft(createCommandOverrideDraft(savedOverrides));
  }, [savedOverrides]);

  const saveMutation = useMutation({
    mutationFn: saveAgentRunSettings,
    onSuccess: async (saved) => {
      setDraft(createCommandOverrideDraft(saved.commandOverrides));
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
  });

  function save() {
    const settings: AgentRunSettings = {
      workingDirectory: APP_COMMAND_OVERRIDE_SETTINGS_KEY,
      agentId: agents[0]?.id ?? "",
      permissionMode: "default",
      modelId: "providerDefault",
      contextSize: "default",
      sessionMode: "new",
      ralphLoop: {
        enabled: false,
        maxIterations: 5,
        delayMs: 0,
        stopOnError: true,
        stopOnPermission: false,
        promptTemplate: "",
      },
      commandOverrides: commandOverridePayload(draft, savedOverrides),
    };
    saveMutation.mutate(settings);
  }

  const loadError =
    agentsQuery.isError || settingsQuery.isError
      ? String(agentsQuery.error ?? settingsQuery.error)
      : null;

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">Settings</h1>
          <p className="text-sm text-muted-foreground">
            ACP agent 프로필(실행 명령·환경변수)을 관리합니다.
          </p>
        </div>
        {onBack && (
          <Button type="button" variant="outline" onClick={onBack}>
            돌아가기
          </Button>
        )}
      </div>

      {settingsQuery.isLoading || agentsQuery.isLoading ? (
        <p className="rounded-md border bg-background px-3 py-2 text-sm text-muted-foreground">
          설정을 불러오는 중입니다.
        </p>
      ) : (
        <AgentCommandOverrideEditor
          agents={agents}
          draft={draft}
          isSaving={saveMutation.isPending}
          loadError={loadError}
          saveError={saveMutation.isError ? String(saveMutation.error) : null}
          onDraftChange={setDraft}
          onSave={save}
        />
      )}
    </div>
  );
}
