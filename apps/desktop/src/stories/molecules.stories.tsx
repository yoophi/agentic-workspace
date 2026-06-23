import type { Meta, StoryObj } from "@storybook/react-vite";
import { CopyIcon, FolderIcon, InfoIcon, SendIcon } from "lucide-react";
import { StickToBottom } from "use-stick-to-bottom";

import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { CodeBlock, CodeBlockCode, CodeBlockGroup } from "@/components/ui/code-block";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator, CommandShortcut } from "@/components/ui/command";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSeparator, FieldSet, FieldTitle } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput, InputGroupText, InputGroupTextarea } from "@/components/ui/input-group";
import { Markdown } from "@/components/ui/markdown";
import { Message, MessageAction, MessageActions, MessageAvatar, MessageContent } from "@/components/ui/message";
import { Popover, PopoverContent, PopoverDescription, PopoverHeader, PopoverTitle, PopoverTrigger } from "@/components/ui/popover";
import { PromptInput, PromptInputAction, PromptInputActions, PromptInputTextarea } from "@/components/ui/prompt-input";
import { ScrollButton } from "@/components/ui/scroll-button";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectSeparator, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Steps, StepsBar, StepsContent, StepsItem, StepsTrigger } from "@/components/ui/steps";
import { Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tool } from "@/components/ui/tool";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

const meta = {
  title: "Atomic Design/Molecules/Registered Components",
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const DataDisplay: Story = {
  render: () => (
    <div className="grid gap-6 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Project Summary</CardTitle>
          <CardDescription>Card, header, action, content and footer.</CardDescription>
          <CardAction>
            <Button variant="outline" size="sm">Edit</Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <Table>
            <TableCaption>Recent worktrees</TableCaption>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell>main</TableCell>
                <TableCell>clean</TableCell>
              </TableRow>
              <TableRow>
                <TableCell>storybook</TableCell>
                <TableCell>dirty</TableCell>
              </TableRow>
            </TableBody>
            <TableFooter>
              <TableRow>
                <TableCell colSpan={2}>2 entries</TableCell>
              </TableRow>
            </TableFooter>
          </Table>
        </CardContent>
        <CardFooter>Updated just now</CardFooter>
      </Card>
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <FolderIcon />
          </EmptyMedia>
          <EmptyTitle>No projects</EmptyTitle>
          <EmptyDescription>Create a project to start a session.</EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button>Create project</Button>
        </EmptyContent>
      </Empty>
    </div>
  ),
};

export const FormsAndSelectors: Story = {
  render: () => (
    <div className="grid max-w-2xl gap-6">
      <FieldSet>
        <FieldLegend>Project</FieldLegend>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="molecule-name">Name</FieldLabel>
            <Input id="molecule-name" defaultValue="ACP Minimal App" />
            <FieldDescription>Displayed in the project list.</FieldDescription>
          </Field>
          <Field orientation="horizontal">
            <FieldContent>
              <FieldTitle>Auto allow</FieldTitle>
              <FieldDescription>Apply safe agent actions automatically.</FieldDescription>
            </FieldContent>
            <Button variant="outline" size="sm">Toggle</Button>
          </Field>
          <FieldError errors={[{ message: "Working directory is required" }]} />
          <FieldSeparator>or</FieldSeparator>
        </FieldGroup>
      </FieldSet>
      <InputGroup>
        <InputGroupAddon>
          <InputGroupText>git</InputGroupText>
        </InputGroupAddon>
        <InputGroupInput placeholder="feature/storybook" />
        <InputGroupAddon align="inline-end">
          <InputGroupButton variant="outline">Create</InputGroupButton>
        </InputGroupAddon>
      </InputGroup>
      <InputGroup>
        <InputGroupTextarea placeholder="Prompt" />
        <InputGroupAddon align="block-end">
          <InputGroupButton size="icon-sm" aria-label="Send">
            <SendIcon />
          </InputGroupButton>
        </InputGroupAddon>
      </InputGroup>
      <Select defaultValue="codex">
        <SelectTrigger>
          <SelectValue placeholder="Agent" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectLabel>Agents</SelectLabel>
            <SelectItem value="codex">Codex</SelectItem>
            <SelectItem value="claude">Claude Code</SelectItem>
            <SelectSeparator />
            <SelectItem value="custom">Custom</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  ),
};

export const OverlaysAndNavigation: Story = {
  render: () => (
    <TooltipProvider>
      <div className="flex flex-wrap items-center gap-3">
        <Dialog>
          <DialogTrigger asChild>
            <Button>Open Dialog</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Project settings</DialogTitle>
              <DialogDescription>Dialog content is composed from header, body and footer slots.</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button>Save</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button variant="destructive">Delete</Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Delete project?</AlertDialogTitle>
              <AlertDialogDescription>This action removes it from the JSON store.</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction variant="destructive">Delete</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline">Popover</Button>
          </PopoverTrigger>
          <PopoverContent>
            <PopoverHeader>
              <PopoverTitle>Remote</PopoverTitle>
              <PopoverDescription>origin points to the default repository.</PopoverDescription>
            </PopoverHeader>
          </PopoverContent>
        </Popover>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Info">
              <InfoIcon />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Tooltip content</TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  ),
};

export const CommandsAndDisclosure: Story = {
  render: () => (
    <div className="grid max-w-3xl gap-6">
      <Command className="rounded-lg border">
        <CommandInput placeholder="Search commands" />
        <CommandList>
          <CommandEmpty>No command found.</CommandEmpty>
          <CommandGroup heading="Project">
            <CommandItem>
              Open project
              <CommandShortcut>⌘O</CommandShortcut>
            </CommandItem>
            <CommandItem>Create worktree</CommandItem>
          </CommandGroup>
          <CommandSeparator />
          <CommandGroup heading="Agent">
            <CommandItem>Start Codex</CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
      <Collapsible defaultOpen>
        <CollapsibleTrigger asChild>
          <Button variant="outline">Toggle details</Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-3 rounded-md border p-3 text-sm text-muted-foreground">
          Collapsible content for advanced details.
        </CollapsibleContent>
      </Collapsible>
      <Steps>
        <StepsItem>
          <StepsTrigger>Initialize</StepsTrigger>
          <StepsContent>Session created.</StepsContent>
        </StepsItem>
        <StepsBar />
        <StepsItem>
          <StepsTrigger>Run</StepsTrigger>
          <StepsContent>Agent is processing the prompt.</StepsContent>
        </StepsItem>
      </Steps>
    </div>
  ),
};

export const ContentAndAgentParts: Story = {
  render: () => (
    <div className="grid gap-6 lg:grid-cols-2">
      <div className="grid gap-4">
        <Markdown>
          {"## Markdown\n\n- Supports **GFM**\n- Renders `inline code`\n\n```ts\nconst ready = true;\n```"}
        </Markdown>
        <CodeBlock>
          <CodeBlockGroup>
            <Button variant="ghost" size="icon-sm" aria-label="Copy"><CopyIcon /></Button>
          </CodeBlockGroup>
          <CodeBlockCode code={"pnpm storybook\npnpm build-storybook"} language="bash" />
        </CodeBlock>
        <Message>
          <MessageAvatar src="" alt="Assistant" fallback="AI" />
          <MessageContent markdown>Storybook catalog is ready.</MessageContent>
          <MessageActions>
            <MessageAction tooltip="Copy">
              <CopyIcon />
            </MessageAction>
          </MessageActions>
        </Message>
      </div>
      <div className="grid gap-4">
        <Tool
          defaultOpen
          toolPart={{
            type: "tool-call",
            toolCallId: "tool-1",
            state: "output-available",
            input: { command: "pnpm check-types" },
            output: { result: "No type errors found." },
          }}
        />
        <PromptInput>
          <PromptInputTextarea placeholder="Ask an agent to work on this project" />
          <PromptInputActions>
            <PromptInputAction tooltip="Send">
              <Button size="icon-sm" aria-label="Send"><SendIcon /></Button>
            </PromptInputAction>
          </PromptInputActions>
        </PromptInput>
        <StickToBottom className="relative h-40 overflow-y-auto rounded-md border p-4">
          <StickToBottom.Content className="grid gap-3">
            {Array.from({ length: 8 }, (_, index) => (
              <p key={index} className="text-sm text-muted-foreground">Scrollable message {index + 1}</p>
            ))}
          </StickToBottom.Content>
          <div className="absolute bottom-3 right-3">
            <ScrollButton />
          </div>
        </StickToBottom>
      </div>
    </div>
  ),
};
