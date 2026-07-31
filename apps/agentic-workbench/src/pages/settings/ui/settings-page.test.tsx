import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { SettingsPage } from "./settings-page";

vi.mock("@/entities/agent-run/api/agent-run-repository", () => ({
  getAgentRunSettings: vi.fn().mockResolvedValue(null),
  listAgents: vi.fn().mockResolvedValue([]),
  saveAgentRunSettings: vi.fn(),
}));

vi.mock("@/features/agent-command-override/ui/agent-command-override-editor", () => ({
  AgentCommandOverrideEditor: ({
    loadError,
    saveError,
  }: {
    loadError: string | null;
    saveError: string | null;
  }) => (
    <div>
      <span>editor</span>
      {loadError && <span>{loadError}</span>}
      {saveError && <span>{saveError}</span>}
    </div>
  ),
}));

vi.mock("@/app/providers/appearance-preferences-provider", () => ({
  useAppearancePreferences: () => ({
    fontSizeStep: 0,
    isHydrated: true,
    error: null,
    setFontSizeStep: vi.fn(),
    adjustFontSizeStep: vi.fn(),
  }),
}));

function renderSettingsPage(element: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return renderToStaticMarkup(
    <QueryClientProvider client={queryClient}>{element}</QueryClientProvider>,
  );
}

describe("SettingsPage", () => {
  it("renders without back navigation in dedicated window mode", () => {
    const html = renderSettingsPage(<SettingsPage />);

    expect(html).toContain("Settings");
    expect(html).toContain("Appearance");
    expect(html).toContain("글꼴 크기");
    expect(html).toContain("ACP agent 프로필");
    expect(html).not.toContain("돌아가기");
  });

  it("can still render a back action for embedded legacy usage", () => {
    const html = renderSettingsPage(<SettingsPage onBack={() => undefined} />);

    expect(html).toContain("돌아가기");
  });
});
