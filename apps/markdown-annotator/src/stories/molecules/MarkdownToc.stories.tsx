import type { Meta, StoryObj } from "@storybook/react-vite";
import { extractTocEntries, parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import { MarkdownToc } from "@yoophi/markdown-annotation-react";

const meta = {
  title: "Molecules/MarkdownToc",
  component: MarkdownToc,
  args: {
    onEntrySelect: (entry) => {
      console.log("select toc entry", entry.blockId, entry.text);
    },
  },
  parameters: {
    layout: "padded",
  },
} satisfies Meta<typeof MarkdownToc>;

export default meta;

type Story = StoryObj<typeof meta>;

function entriesFromMarkdown(markdown: string) {
  return extractTocEntries(parseMarkdownToBlocks(markdown));
}

export const Basic: Story = {
  args: {
    entries: entriesFromMarkdown(`# 문서 제목

## 배경

### 문제 정의

### 목표

## 설계

### 대안 비교

#### h4는 TOC에 포함되지 않음

## 결론
`),
    className: "w-64",
  },
};

export const LongList: Story = {
  render: (args) => (
    <div className="h-80 w-64 overflow-y-auto rounded-lg border p-2">
      <MarkdownToc {...args} />
    </div>
  ),
  args: {
    entries: entriesFromMarkdown(
      Array.from({ length: 40 }, (_, index) =>
        [`## Section ${index + 1}`, "", `### Detail ${index + 1}`, ""].join("\n"),
      ).join("\n"),
    ),
  },
};

export const StartsAtH3: Story = {
  args: {
    entries: entriesFromMarkdown(`### h1 없이 시작하는 문서

### level 절대값 기준 들여쓰기 유지
`),
    className: "w-64",
  },
};

export const InlineFormattedHeadings: Story = {
  args: {
    entries: entriesFromMarkdown(`# **Bold** 제목

## \`inline code\` 포함

### [링크 텍스트](https://example.com) heading

## ~~취소선~~ 과 _italic_
`),
    className: "w-64",
  },
};

export const EmptyEntries: Story = {
  render: (args) => (
    <div className="w-64 rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
      <MarkdownToc {...args} />
      entries가 비어 있으면 위에 아무것도 렌더되지 않습니다.
    </div>
  ),
  args: {
    entries: entriesFromMarkdown("h1~h3 heading이 없는 문서의 본문 텍스트"),
  },
};
