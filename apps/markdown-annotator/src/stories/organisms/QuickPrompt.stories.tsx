import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  buildDocumentQuickPromptContext,
  buildSelectionQuickPromptContext,
} from "@yoophi/markdown-annotation-core";
import { QuickPromptDialog } from "@/features/quick-prompt/ui/QuickPromptDialog";
import {
  createAvailableQuickPromptTarget,
  standaloneQuickPromptTarget,
} from "@/features/quick-prompt/model/quick-prompt-target";

const selectionContext = buildSelectionQuickPromptContext({
  sourceLabel: "review.md",
  documentPath: "/workspace/review.md",
  documentRevisionLabel: "review.md:128:4",
  selectedText: "이 문단의 표현이 모호합니다.",
  anchors: [
    {
      blockId: "block-1",
      startLine: 3,
      endLine: 3,
      startOffset: 0,
      endOffset: 14,
      selectedText: "이 문단의 표현이 모호합니다.",
    },
  ],
});

const documentContext = buildDocumentQuickPromptContext({
  sourceLabel: "review.md",
  documentPath: "/workspace/review.md",
  documentRevisionLabel: "review.md:420:12",
  markdownText: "# Review\n\n문서 전체를 검토합니다.\n\n- 항목 A\n- 항목 B",
});

const longContext = buildDocumentQuickPromptContext({
  sourceLabel: "long.md",
  documentPath: "/workspace/long.md",
  documentRevisionLabel: "long.md:8000:120",
  markdownText: Array.from({ length: 80 }, (_, index) => `## Section ${index + 1}\n\nLong body text.`).join("\n\n"),
  maxContentLength: 240,
});

const meta = {
  title: "Organisms/QuickPrompt",
  component: QuickPromptDialog,
  args: {
    open: true,
    promptText: "이 컨텍스트를 검토해줘.",
    onPromptTextChange: () => undefined,
    onOpenChange: () => undefined,
    target: standaloneQuickPromptTarget,
  },
  parameters: {
    layout: "centered",
  },
} satisfies Meta<typeof QuickPromptDialog>;

export default meta;

type Story = StoryObj<typeof meta>;

export const SelectionUnavailable: Story = {
  args: {
    context: selectionContext,
  },
};

export const SelectionReady: Story = {
  args: {
    context: selectionContext,
    target: createAvailableQuickPromptTarget("Active agent session"),
  },
};

export const DocumentContext: Story = {
  args: {
    context: documentContext,
    target: createAvailableQuickPromptTarget("Active agent session"),
  },
};

export const LongContextReduced: Story = {
  args: {
    context: longContext,
    target: createAvailableQuickPromptTarget("Active agent session"),
  },
};

export const EmptyContextBlocked: Story = {
  args: {
    context: null,
    promptText: "",
  },
};

export const SendingState: Story = {
  args: {
    context: selectionContext,
    target: createAvailableQuickPromptTarget("Active agent session"),
    status: "sending",
  },
};
