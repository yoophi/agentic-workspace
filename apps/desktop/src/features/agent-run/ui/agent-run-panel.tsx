import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { BotIcon, PlayIcon, SquareIcon, XIcon } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  cancelAgentRun,
  listenRunEvents,
  listAgents,
  listProviderSessions,
  sendPromptToRun,
  startAgentRun,
} from "@/entities/agent-run/api/agent-run-repository";
import { agentRunQueryKeys } from "@/entities/agent-run/api/query-keys";
import {
  eventGroups,
  appendOneTimelineItem,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { EventGroup, ProviderSession, TimelineItem } from "@/entities/agent-run/model";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CodeBlock, CodeBlockCode } from "@/components/ui/code-block";
import { Message, MessageAvatar } from "@/components/ui/message";
import {
  PromptInput,
  PromptInputAction,
  PromptInputActions,
  PromptInputTextarea,
} from "@/components/ui/prompt-input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Steps, StepsContent, StepsItem, StepsTrigger } from "@/components/ui/steps";
import { SystemMessage } from "@/components/ui/system-message";
import { cn } from "@/lib/utils";

type AgentRunPanelProps = {
  workingDirectory: string;
};

const defaultPrompt = "";

type QueuedPrompt = {
  id: string;
  text: string;
};

export function AgentRunPanel({ workingDirectory }: AgentRunPanelProps) {
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [sessionMode, setSessionMode] = useState<"new" | "reuse">("new");
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isAwaitingPromptResponse, setIsAwaitingPromptResponse] = useState(false);
  const [queuedPrompts, setQueuedPrompts] = useState<QueuedPrompt[]>([]);
  const [filter, setFilter] = useState<EventGroup | "all">("all");
  const [items, setItems] = useState<TimelineItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);
  const activeRunIdRef = useRef<string | null>(null);

  const agentsQuery = useQuery({
    queryKey: agentRunQueryKeys.agents,
    queryFn: listAgents,
  });
  const agents = agentsQuery.data ?? [];

  const sessionsQuery = useQuery({
    queryKey: agentRunQueryKeys.sessions(selectedAgentId, workingDirectory),
    queryFn: () => listProviderSessions(selectedAgentId, workingDirectory),
    enabled: sessionMode === "reuse" && Boolean(selectedAgentId),
  });
  const sessions = sessionsQuery.data ?? [];

  useEffect(() => {
    if (!selectedAgentId && agents[0]) {
      setSelectedAgentId(agents[0].id);
    }
  }, [agents, selectedAgentId]);

  // agent를 바꾸면 이전 provider의 세션 선택은 더 이상 유효하지 않다.
  useEffect(() => {
    setSelectedSessionId("");
  }, [selectedAgentId]);

  useEffect(() => {
    activeRunIdRef.current = activeRunId;
  }, [activeRunId]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenRunEvents((envelope) => {
      setItems((currentItems) =>
        appendOneTimelineItem(currentItems, toTimelineItem(envelope.runId, envelope.event)),
      );

      if (envelope.runId !== activeRunIdRef.current) {
        return;
      }

      if (envelope.event.type === "error") {
        setIsAwaitingPromptResponse(false);
        setQueuedPrompts([]);
        setIsRunning(false);
        activeRunIdRef.current = null;
        setActiveRunId(null);
        return;
      }

      if (envelope.event.type === "lifecycle") {
        if (envelope.event.status === "promptSent") {
          setIsAwaitingPromptResponse(true);
        }
        if (envelope.event.status === "promptCompleted") {
          setIsAwaitingPromptResponse(false);
        }
        if (["completed", "cancelled"].includes(envelope.event.status)) {
          setIsAwaitingPromptResponse(false);
          setQueuedPrompts([]);
          setIsRunning(false);
          activeRunIdRef.current = null;
          setActiveRunId(null);
        }
      }
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [items]);

  useEffect(() => {
    if (!activeRunId || !isRunning || isAwaitingPromptResponse || queuedPrompts.length === 0) {
      return;
    }

    const nextPrompt = queuedPrompts[0];
    setIsAwaitingPromptResponse(true);
    setQueuedPrompts((current) => current.slice(1));
    void sendPromptToRun(activeRunId, nextPrompt.text).catch((caughtError) => {
      setQueuedPrompts((current) => [nextPrompt, ...current]);
      setIsAwaitingPromptResponse(false);
      setError(String(caughtError));
    });
  }, [activeRunId, isAwaitingPromptResponse, isRunning, queuedPrompts]);

  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId);
  const visibleItems = useMemo(
    () => (filter === "all" ? items : items.filter((item) => item.group === filter)),
    [filter, items],
  );
  const canStartRun = Boolean(
    selectedAgentId &&
      prompt.trim() &&
      !isRunning &&
      (sessionMode === "new" || selectedSessionId),
  );
  const canQueuePrompt = Boolean(activeRunId && isRunning && prompt.trim());
  const canCancel = Boolean(activeRunId && isRunning);

  async function run() {
    const goal = prompt.trim();
    if (!selectedAgentId || !goal) {
      return;
    }

    const runId = crypto.randomUUID();
    setError(null);
    setItems([]);
    setQueuedPrompts([]);
    activeRunIdRef.current = runId;
    setActiveRunId(runId);
    setIsRunning(true);
    setIsAwaitingPromptResponse(true);
    setPrompt(defaultPrompt);

    const reuseSession = sessionMode === "reuse" && Boolean(selectedSessionId);

    try {
      await startAgentRun({
        runId,
        goal,
        agentId: selectedAgentId,
        cwd: workingDirectory,
        stdioBufferLimitMb: 50,
        autoAllow: true,
        ...(reuseSession
          ? { resumeSessionId: selectedSessionId, resumePolicy: "resumeIfAvailable" }
          : {}),
      });
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(goal);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activeRunIdRef.current = null;
      setActiveRunId(null);
    }
  }

  function enqueuePrompt() {
    const nextPrompt = prompt.trim();
    if (!nextPrompt) {
      return;
    }

    setQueuedPrompts((current) => [...current, { id: crypto.randomUUID(), text: nextPrompt }]);
    setPrompt(defaultPrompt);
  }

  async function cancel() {
    if (!activeRunId) {
      return;
    }

    try {
      await cancelAgentRun(activeRunId);
    } catch (caughtError) {
      setError(String(caughtError));
    } finally {
      setQueuedPrompts([]);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activeRunIdRef.current = null;
      setActiveRunId(null);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div className="flex flex-col gap-1.5">
            <CardTitle className="flex items-center gap-2">
              <BotIcon />
              Agentic coding
            </CardTitle>
            <CardDescription>
              선택한 worktree를 작업 디렉토리로 사용해 ACP agent를 실행합니다.
            </CardDescription>
          </div>
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">Agent</span>
              <Select
                value={selectedAgentId}
                onValueChange={setSelectedAgentId}
                disabled={isRunning || agentsQuery.isLoading}
              >
                <SelectTrigger className="w-56">
                  <SelectValue placeholder="Agent 선택" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {agents.map((agent) => (
                      <SelectItem key={agent.id} value={agent.id}>
                        {agent.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">세션</span>
              <div className="flex gap-1.5">
                <Button
                  type="button"
                  size="sm"
                  variant={sessionMode === "new" ? "default" : "outline"}
                  disabled={isRunning}
                  onClick={() => setSessionMode("new")}
                >
                  새 세션
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={sessionMode === "reuse" ? "default" : "outline"}
                  disabled={isRunning}
                  onClick={() => setSessionMode("reuse")}
                >
                  기존 세션 재사용
                </Button>
              </div>
              {sessionMode === "reuse" && (
                <Select
                  value={selectedSessionId}
                  onValueChange={setSelectedSessionId}
                  disabled={isRunning || sessionsQuery.isLoading || sessions.length === 0}
                >
                  <SelectTrigger className="w-56">
                    <SelectValue
                      placeholder={
                        sessionsQuery.isLoading
                          ? "세션 불러오는 중…"
                          : sessions.length === 0
                            ? "재사용 가능한 세션 없음"
                            : "재개할 세션 선택"
                      }
                    />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {sessions.map((session) => (
                        <SelectItem key={session.id} value={session.id}>
                          {formatSessionLabel(session)}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              )}
              {sessionMode === "reuse" &&
                !sessionsQuery.isLoading &&
                sessions.length === 0 && (
                  <span className="text-xs text-muted-foreground">
                    이 worktree에서 해당 agent의 기존 세션을 찾지 못했습니다.
                  </span>
                )}
            </div>
          </div>
        </div>
      </CardHeader>
      <CardContent className="grid min-h-[680px] grid-rows-[auto_minmax(0,1fr)_auto] gap-4">
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={isRunning ? "default" : "secondary"}>
              {isRunning ? "Running" : "Idle"}
            </Badge>
            <span className="break-all font-mono text-xs text-muted-foreground">
              cwd={workingDirectory}
            </span>
          </div>
          {selectedAgent && (
            <span className="break-all font-mono text-xs text-muted-foreground">
              command={selectedAgent.command}
            </span>
          )}
          {error && (
            <SystemMessage variant="error" fill>
              {error}
            </SystemMessage>
          )}
        </div>

        <div className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] rounded-lg border bg-background">
          <div className="flex flex-wrap gap-1.5 border-b p-3" role="tablist" aria-label="ACP event filter">
            {eventGroups.map((group) => (
              <Button
                key={group.id}
                type="button"
                size="sm"
                variant={filter === group.id ? "default" : "outline"}
                onClick={() => setFilter(group.id)}
              >
                {group.label}
              </Button>
            ))}
          </div>
          <div className="min-h-0 overflow-auto p-4" role="log" aria-live="polite">
            {visibleItems.length === 0 ? (
              <div className="grid min-h-[320px] place-items-center rounded-lg border border-dashed bg-muted/30 text-sm text-muted-foreground">
                ACP 응답이 아직 없습니다.
              </div>
            ) : (
              <div className="flex flex-col gap-3">
                {visibleItems.map((item) => (
                  <RunEventItem key={item.id} item={item} />
                ))}
              </div>
            )}
            <div ref={endRef} />
          </div>
        </div>

        <PromptInput
          value={prompt}
          onValueChange={setPrompt}
          onSubmit={() => {
            if (isRunning) {
              enqueuePrompt();
              return;
            }
            if (canStartRun) {
              void run();
            }
          }}
          isLoading={isRunning}
          className="rounded-lg"
        >
          <PromptInputTextarea placeholder="선택한 worktree에서 실행할 작업을 입력하세요." />
          {queuedPrompts.length > 0 && (
            <div className="flex flex-col gap-2 px-2 pb-2">
              <div className="text-xs text-muted-foreground">대기 중인 prompt {queuedPrompts.length}개</div>
              <div className="flex flex-col gap-2">
                {queuedPrompts.map((queuedPrompt, index) => (
                  <div
                    key={queuedPrompt.id}
                    className="flex items-start justify-between gap-3 rounded-md border bg-muted/40 px-3 py-2"
                  >
                    <div className="min-w-0 flex-1 text-sm">
                      <span className="mr-2 text-xs text-muted-foreground">#{index + 1}</span>
                      <span className="whitespace-pre-wrap break-words">{queuedPrompt.text}</span>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="size-7"
                      onClick={() => {
                        setQueuedPrompts((current) =>
                          current.filter((item) => item.id !== queuedPrompt.id),
                        );
                      }}
                    >
                      <XIcon className="size-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className="flex items-center justify-between gap-3 px-2 pb-1">
            <span className="text-xs text-muted-foreground">
              {isRunning
                ? isAwaitingPromptResponse
                  ? "현재 prompt 처리 중입니다. Enter로 다음 prompt를 queue에 추가합니다."
                  : "다음 prompt를 바로 보낼 수 있습니다. Enter로 queue에 추가합니다."
                : "Enter로 실행, Shift+Enter로 줄바꿈"}
            </span>
            <PromptInputActions>
              {isRunning ? (
                <>
                  <PromptInputAction tooltip="Queue prompt">
                    <Button type="button" size="sm" disabled={!canQueuePrompt} onClick={enqueuePrompt}>
                      <PlayIcon data-icon="inline-start" />
                      Queue
                    </Button>
                  </PromptInputAction>
                  <PromptInputAction tooltip="Cancel run">
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      disabled={!canCancel}
                      onClick={() => void cancel()}
                    >
                      <SquareIcon data-icon="inline-start" />
                      Cancel
                    </Button>
                  </PromptInputAction>
                </>
              ) : (
                <PromptInputAction tooltip="Start run">
                  <Button type="button" size="sm" disabled={!canStartRun} onClick={() => void run()}>
                    <PlayIcon data-icon="inline-start" />
                    Run
                  </Button>
                </PromptInputAction>
              )}
            </PromptInputActions>
          </div>
        </PromptInput>
      </CardContent>
    </Card>
  );
}

function RunEventItem({ item }: { item: TimelineItem }) {
  if (item.group === "tool_call/tool_result") {
    return <ToolStep item={item} />;
  }

  if (item.event.type === "lifecycle") {
    return <LifecycleStep item={item} />;
  }

  if (item.group === "lifecycle" || item.group === "error" || item.group === "permission") {
    return (
      <SystemMessage
        variant={item.group === "error" ? "error" : item.tone === "warning" ? "warning" : "action"}
        fill={item.group === "error"}
      >
        <span className="font-medium">{item.title}</span>
        <span className="ml-2 break-words">{item.body}</span>
      </SystemMessage>
    );
  }

  if (item.group === "raw") {
    return (
      <CodeBlock className="rounded-lg">
        <CodeBlockCode code={item.body} language="json" />
      </CodeBlock>
    );
  }

  return (
    <Message className={cn(item.group === "thought" && "opacity-80")}>
      <MessageAvatar src="" alt={item.group} fallback={item.group === "assistant/message" ? "AI" : "•"} />
      <div className="min-w-0 flex-1 rounded-lg bg-secondary p-2 text-foreground">
        {item.group === "assistant/message" || item.group === "thought" ? (
          <StreamingMarkdown content={item.body} />
        ) : (
          <pre className="m-0 whitespace-pre-wrap break-words font-mono text-sm leading-6">
            {item.body}
          </pre>
        )}
      </div>
    </Message>
  );
}

function LifecycleStep({ item }: { item: TimelineItem }) {
  const status = item.event.type === "lifecycle" ? item.event.status : "started";
  const lines = item.body.split("\n").filter(Boolean);

  return (
    <Steps className="rounded-lg border bg-background px-3 py-2" defaultOpen={false}>
      <StepsTrigger className="items-start">
        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span className="font-medium text-foreground">Agent run</span>
          <span className={cn("rounded-full px-2 py-0.5 text-xs font-medium", lifecycleStatusClassName(status))}>
            {lifecycleStatusLabel(status)}
          </span>
        </span>
      </StepsTrigger>
      <StepsContent>
        {lines.map((line, index) => {
          const [lineStatus, ...messageParts] = line.split(": ");
          const message = messageParts.join(": ");
          return (
            <StepsItem
              key={`${line}-${index}`}
              className={cn(index === lines.length - 1 && isLifecycleTerminal(lineStatus) && "text-foreground")}
            >
              <span className="font-medium text-foreground">{lifecycleStatusLabel(lineStatus)}</span>
              {message && <span className="ml-2 break-words">{message}</span>}
            </StepsItem>
          );
        })}
      </StepsContent>
    </Steps>
  );
}

function ToolStep({ item }: { item: TimelineItem }) {
  const tool = item.event.type === "tool" ? item.event : null;
  const status = tool?.status || (item.tone === "success" ? "completed" : item.tone === "danger" ? "failed" : "running");
  const locations = tool?.locations ?? [];
  const toolCallId = tool?.toolCallId;

  return (
    <div className="space-y-4">
    <Steps className="" defaultOpen={false}>
      <StepsTrigger>
        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span className="">{item.title || "tool"}</span>
          <span className={cn("rounded-full px-2 py-0.5 text-xs font-medium", toolStatusClassName(status))}>
            {toolStatusLabel(status)}
          </span>
        </span>
      </StepsTrigger>
      <StepsContent>
        {locations.map((path) => (
          <StepsItem key={path}>
            <span className="font-medium text-foreground">path</span>{" "}
            <code className="break-all rounded bg-background px-1.5 py-0.5 font-mono text-xs">{path}</code>
          </StepsItem>
        ))}
        {!toolCallId && locations.length === 0 && item.body && (
          <StepsItem className="whitespace-pre-wrap break-words font-mono text-xs">{item.body}</StepsItem>
        )}
      </StepsContent>
    </Steps>
    </div>
  );
}

function formatSessionLabel(session: ProviderSession) {
  const title = session.title?.trim() || session.id.slice(0, 8);
  const parts = [title, `${session.messageCount} msgs`];
  if (session.updatedAt) {
    const when = new Date(session.updatedAt);
    if (!Number.isNaN(when.getTime())) {
      parts.push(when.toLocaleString());
    }
  }
  return parts.join(" · ");
}

function lifecycleStatusLabel(status: string) {
  if (status === "started") return "Started";
  if (status === "initialized") return "Initialized";
  if (status === "sessionCreated") return "Session created";
  if (status === "promptSent") return "Prompt sent";
  if (status === "promptCompleted") return "Prompt completed";
  if (status === "completed") return "Completed";
  if (status === "cancelled") return "Cancelled";
  return status || "Lifecycle";
}

function lifecycleStatusClassName(status: string) {
  if (status === "completed" || status === "promptCompleted") {
    return "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300";
  }
  if (status === "cancelled") {
    return "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300";
  }
  return "bg-primary/10 text-primary";
}

function isLifecycleTerminal(status: string) {
  return status === "completed" || status === "promptCompleted" || status === "cancelled";
}

function toolStatusLabel(status: string) {
  if (status === "completed") return "Completed";
  if (status === "failed") return "Failed";
  if (status === "in_progress") return "In progress";
  if (status === "pending") return "In progress";
  if (status === "running") return "Running";
  return status || "Tool";
}

function toolStatusClassName(status: string) {
  if (status === "completed") return "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300";
  if (status === "failed") return "bg-destructive/10 text-destructive";
  if (status === "in_progress" || status === "pending" || status === "running") return "bg-primary/10 text-primary";
  return "bg-secondary text-secondary-foreground";
}

function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="min-w-0 break-words text-sm leading-6">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => (
            <h1 className="mb-2 mt-4 text-2xl font-semibold tracking-tight first:mt-0">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="mb-2 mt-4 text-xl font-semibold tracking-tight first:mt-0">
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="mb-2 mt-3 text-lg font-semibold tracking-tight first:mt-0">
              {children}
            </h3>
          ),
          h4: ({ children }) => (
            <h4 className="mb-2 mt-3 text-base font-semibold tracking-tight first:mt-0">
              {children}
            </h4>
          ),
          p: ({ children }) => <p className="my-2 first:mt-0 last:mb-0">{children}</p>,
          ul: ({ children }) => <ul className="my-2 list-disc pl-5">{children}</ul>,
          ol: ({ children }) => <ol className="my-2 list-decimal pl-5">{children}</ol>,
          li: ({ children }) => <li className="mt-1">{children}</li>,
          blockquote: ({ children }) => (
            <blockquote className="my-3 border-l-4 pl-4 text-muted-foreground">
              {children}
            </blockquote>
          ),
          code: ({ children, className }) => {
            const isBlockCode = className?.includes("language-");
            return (
              <code
                className={cn(
                  "font-mono",
                  isBlockCode
                    ? "text-sm"
                    : "rounded bg-muted px-1.5 py-0.5 text-[0.92em]",
                  className,
                )}
              >
                {children}
              </code>
            );
          },
          pre: ({ children }) => (
            <pre className="my-3 overflow-x-auto rounded-md border bg-muted p-3">
              {children}
            </pre>
          ),
          a: ({ children, href }) => (
            <a className="font-medium text-primary underline underline-offset-4" href={href} rel="noreferrer" target="_blank">
              {children}
            </a>
          ),
          hr: () => <hr className="my-4 border-border" />,
          table: ({ children }) => (
            <div className="my-3 overflow-x-auto">
              <table className="w-full border-collapse text-sm">{children}</table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border bg-muted px-2 py-1 text-left font-semibold align-top">
              {children}
            </th>
          ),
          td: ({ children }) => <td className="border px-2 py-1 align-top">{children}</td>,
          img: ({ alt, src }) => <img className="h-auto max-w-full rounded-md" alt={alt ?? ""} src={src} />,
        }}
      >
        {normalizeStreamingMarkdown(content)}
      </ReactMarkdown>
    </div>
  );
}

function normalizeStreamingMarkdown(content: string) {
  const fenceMatches = content.match(/```/g);
  if (fenceMatches && fenceMatches.length % 2 === 1) {
    const suffix = content.endsWith("\n") ? "```" : "\n```";
    return `${content}${suffix}`;
  }
  return content;
}
