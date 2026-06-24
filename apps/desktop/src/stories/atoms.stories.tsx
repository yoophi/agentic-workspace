import type { Meta, StoryObj } from "@storybook/react-vite";
import { DownloadIcon, UserIcon } from "lucide-react";

import { Avatar, AvatarBadge, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { CircularLoader, ClassicLoader, DotsLoader, Loader, PulseDotLoader, PulseLoader, TerminalLoader, TextBlinkLoader, TextDotsLoader, TextShimmerLoader, TypingLoader, WaveLoader, BarsLoader } from "@/components/ui/loader";
import { Separator } from "@/components/ui/separator";
import { SystemMessage } from "@/components/ui/system-message";
import { Textarea } from "@/components/ui/textarea";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

const meta = {
  title: "Atomic Design/Atoms/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "Primitive UI controls and single-purpose visual elements.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const Buttons: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button>Default</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="outline">Outline</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="destructive">Destructive</Button>
      <Button variant="link">Link</Button>
      <Button size="icon" aria-label="Download">
        <DownloadIcon />
      </Button>
      <Button disabled>Disabled</Button>
    </div>
  ),
};

export const BadgesAndAvatars: Story = {
  render: () => (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap gap-2">
        <Badge>Default</Badge>
        <Badge variant="secondary">Secondary</Badge>
        <Badge variant="outline">Outline</Badge>
        <Badge variant="destructive">Destructive</Badge>
      </div>
      <div className="flex items-center gap-5">
        <Avatar>
          <AvatarImage src="https://github.com/shadcn.png" alt="User" />
          <AvatarFallback>SC</AvatarFallback>
          <AvatarBadge />
        </Avatar>
        <Avatar size="lg">
          <AvatarFallback>
            <UserIcon />
          </AvatarFallback>
        </Avatar>
        <AvatarGroup>
          <Avatar>
            <AvatarFallback>CD</AvatarFallback>
          </Avatar>
          <Avatar>
            <AvatarFallback>AI</AvatarFallback>
          </Avatar>
          <AvatarGroupCount>+3</AvatarGroupCount>
        </AvatarGroup>
      </div>
    </div>
  ),
};

export const Inputs: Story = {
  render: () => (
    <div className="grid max-w-xl gap-5">
      <div className="grid gap-2">
        <Label htmlFor="atom-email">Email</Label>
        <Input id="atom-email" type="email" placeholder="you@example.com" />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="atom-notes">Notes</Label>
        <Textarea id="atom-notes" placeholder="작업 메모를 입력하세요." />
      </div>
      <Separator />
      <SystemMessage variant="action" fill>
        저장이 완료되었습니다.
      </SystemMessage>
    </div>
  ),
};

export const EllipsisPopoverTexts: Story = {
  render: () => (
    <div className="grid max-w-xl gap-5">
      <div className="grid gap-2">
        <Label>Project path</Label>
        <div className="w-72 rounded-md border bg-background px-3 py-2">
          <EllipsisPopoverText
            value="/Users/yoophi/project/worktrees/acp-minimal-app/feature/ellipsis-popover-text"
            className="font-mono text-xs text-muted-foreground"
            contentClassName="font-mono text-xs"
          />
        </div>
      </div>
      <div className="grid gap-2">
        <Label>Remote URL</Label>
        <div className="w-80 rounded-md border bg-background px-3 py-2">
          <EllipsisPopoverText
            value="git@github.com:yoophi/acp-minimal-app-with-a-very-long-repository-name.git"
            className="font-mono text-xs text-muted-foreground"
            contentClassName="font-mono text-xs"
          />
        </div>
      </div>
      <Button type="button" variant="outline" className="w-64 justify-start">
        <EllipsisPopoverText
          value="feature/add-ellipsis-popover-text-to-long-values"
          className="flex-1 text-left"
          focusable={false}
        />
      </Button>
    </div>
  ),
};

export const Loaders: Story = {
  render: () => (
    <div className="grid gap-5 md:grid-cols-3">
      <CircularLoader />
      <ClassicLoader />
      <PulseLoader />
      <PulseDotLoader />
      <DotsLoader />
      <TypingLoader />
      <WaveLoader />
      <BarsLoader />
      <TerminalLoader />
      <TextBlinkLoader />
      <TextShimmerLoader />
      <TextDotsLoader />
      <Loader variant="dots" />
    </div>
  ),
};
