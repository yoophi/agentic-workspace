import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, type ComponentProps } from "react";
import { createRoot } from "react-dom/client";

import type { RunEventEnvelope } from "@/entities/agent-run/model/types";

import { AgentRunPanel } from "./agent-run-panel";

type AgentRunPanelProps = ComponentProps<typeof AgentRunPanel>;
type PromptKeyOptions = KeyboardEventInit & {
  selectionStart?: number;
  selectionEnd?: number;
};

type AgentRunPanelHarness = {
  container: HTMLDivElement;
  queryClient: QueryClient;
  enterPrompt: (value: string) => Promise<void>;
  pressPromptKey: (key: string, options?: PromptKeyOptions) => Promise<void>;
  promptValue: () => string;
  promptSelection: () => { start: number; end: number };
  selectSuggestionWithPointer: (name: string) => Promise<void>;
  selectOption: (label: string, option: string) => Promise<void>;
  clickButton: (name: string) => Promise<void>;
  emitRunEvent: (envelope: RunEventEnvelope) => Promise<void>;
  rerender: (props: Partial<AgentRunPanelProps>) => Promise<void>;
  unmount: () => Promise<void>;
};

const activeHarnesses = new Set<AgentRunPanelHarness>();

installBrowserTestDoubles();

export async function renderAgentRunPanel(
  initialProps: AgentRunPanelProps,
): Promise<AgentRunPanelHarness> {
  const container = document.createElement("div");
  document.body.append(container);

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: Number.POSITIVE_INFINITY,
      },
      mutations: { retry: false },
    },
  });
  const root = createRoot(container);
  let props = initialProps;

  const render = async () => {
    await act(async () => {
      root.render(
        <QueryClientProvider client={queryClient}>
          <AgentRunPanel {...props} />
        </QueryClientProvider>,
      );
      await Promise.resolve();
    });
  };

  const harness: AgentRunPanelHarness = {
    container,
    queryClient,
    enterPrompt: async (value) => {
      await waitForAgentRunPanel(() => Boolean(container.querySelector("textarea")));
      const textarea = getPromptTextarea(container);
      await act(async () => {
        setNativeTextareaValue(textarea, value);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await act(async () => {
        textarea.setSelectionRange(value.length, value.length);
        textarea.dispatchEvent(
          new KeyboardEvent("keyup", {
            key: value[value.length - 1] ?? "Unidentified",
            bubbles: true,
          }),
        );
      });
    },
    pressPromptKey: async (key, options = {}) => {
      const textarea = getPromptTextarea(container);
      const {
        selectionStart = textarea.value.length,
        selectionEnd = selectionStart,
        ...keyboardOptions
      } = options;
      await act(async () => {
        textarea.focus();
        textarea.setSelectionRange(selectionStart, selectionEnd);
        textarea.dispatchEvent(
          new KeyboardEvent("keydown", {
            ...keyboardOptions,
            key,
            bubbles: true,
            cancelable: true,
          }),
        );
      });
    },
    promptValue: () => getPromptTextarea(container).value,
    promptSelection: () => {
      const textarea = getPromptTextarea(container);
      return { start: textarea.selectionStart, end: textarea.selectionEnd };
    },
    selectSuggestionWithPointer: async (name) => {
      const option = [...container.querySelectorAll<HTMLButtonElement>("[role='option']")].find(
        (candidate) => candidate.textContent?.includes(name),
      );
      if (!option) {
        throw new Error(`AgentRunPanel autocomplete option was not rendered: ${name}`);
      }
      await act(async () => {
        option.dispatchEvent(new Event("pointerdown", { bubbles: true, cancelable: true }));
      });
    },
    selectOption: async (label, option) => {
      const trigger = document.querySelector<HTMLButtonElement>(`button[aria-label='${label}']`);
      if (!trigger) {
        throw new Error(`AgentRunPanel select was not rendered: ${label}`);
      }
      await act(async () => {
        trigger.dispatchEvent(
          new PointerEvent("pointerdown", {
            bubbles: true,
            cancelable: true,
            button: 0,
            pointerType: "mouse",
          }),
        );
      });
      await waitForAgentRunPanel(() =>
        [...document.querySelectorAll<HTMLElement>("[role='option']")].some(
          (candidate) => candidate.textContent?.trim() === option,
        ),
      );
      const item = [...document.querySelectorAll<HTMLElement>("[role='option']")].find(
        (candidate) => candidate.textContent?.trim() === option,
      );
      if (!item) {
        throw new Error(`AgentRunPanel option was not rendered: ${option}`);
      }
      await act(async () => {
        item.click();
      });
      await waitForAgentRunPanel(() => trigger.textContent?.includes(option) ?? false);
    },
    clickButton: async (name) => {
      await waitForAgentRunPanel(() => {
        const button = findButton(container, name);
        return Boolean(button && !button.disabled);
      });
      const button = findButton(container, name);
      if (!button) {
        throw new Error(`AgentRunPanel button was not rendered: ${name}`);
      }
      await act(async () => {
        button.click();
      });
    },
    emitRunEvent: async (envelope) => {
      await act(async () => {
        window.dispatchEvent(
          new CustomEvent<RunEventEnvelope>("agent-run-event-fallback", {
            detail: envelope,
          }),
        );
      });
    },
    rerender: async (nextProps) => {
      props = { ...props, ...nextProps };
      await render();
    },
    unmount: async () => {
      if (!activeHarnesses.delete(harness)) {
        return;
      }
      await act(async () => {
        root.unmount();
      });
      queryClient.clear();
      container.remove();
    },
  };

  activeHarnesses.add(harness);
  await render();
  return harness;
}

export async function waitForAgentRunPanel(
  predicate: () => boolean,
  timeoutMs = 2_000,
) {
  const startedAt = Date.now();
  while (!predicate()) {
    if (Date.now() - startedAt >= timeoutMs) {
      throw new Error("Timed out waiting for AgentRunPanel state.");
    }
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });
  }
}

export async function cleanupAgentRunPanelTests() {
  await Promise.all([...activeHarnesses].map((harness) => harness.unmount()));
  document.body.innerHTML = "";
}

function installBrowserTestDoubles() {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;

  if (!("ResizeObserver" in globalThis)) {
    globalThis.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  }

  Element.prototype.scrollIntoView ??= () => undefined;
  Element.prototype.hasPointerCapture ??= () => false;
  Element.prototype.setPointerCapture ??= () => undefined;
  Element.prototype.releasePointerCapture ??= () => undefined;
}

function findButton(container: HTMLElement, name: string) {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent?.trim() === name,
  );
}

function getPromptTextarea(container: HTMLElement) {
  const textarea = container.querySelector("textarea");
  if (!(textarea instanceof HTMLTextAreaElement)) {
    throw new Error("AgentRunPanel prompt textarea was not rendered.");
  }
  return textarea;
}

function setNativeTextareaValue(textarea: HTMLTextAreaElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLTextAreaElement.prototype,
    "value",
  )?.set;
  if (!setter) {
    throw new Error("HTMLTextAreaElement value setter is unavailable.");
  }
  setter.call(textarea, value);
}
