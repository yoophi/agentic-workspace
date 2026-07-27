import { useEffect, useMemo, useState } from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import type {
  AgentPanelEndpoint,
  AgentPromptDelivery,
} from "@/entities/agent-run/model/agent-exchange";

const MAX_MESSAGE_BYTES = 16_384;

type AgentPeerMessageDialogProps = {
  open: boolean;
  sourcePanelId: string | null;
  peers: AgentPanelEndpoint[];
  onOpenChange: (open: boolean) => void;
  onSubmit: (input: {
    sourcePanelId: string;
    targetPanelId: string;
    message: string;
    delivery: AgentPromptDelivery;
  }) => Promise<void> | void;
};

export function AgentPeerMessageDialog({
  open,
  sourcePanelId,
  peers,
  onOpenChange,
  onSubmit,
}: AgentPeerMessageDialogProps) {
  const [targetPanelId, setTargetPanelId] = useState("");
  const [delivery, setDelivery] = useState<AgentPromptDelivery>("queue");
  const [message, setMessage] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const messageBytes = useMemo(() => new TextEncoder().encode(message).length, [message]);

  useEffect(() => {
    if (open && !peers.some((peer) => peer.panelId === targetPanelId)) {
      setTargetPanelId(peers[0]?.panelId ?? "");
    }
  }, [open, peers, targetPanelId]);

  const valid =
    Boolean(sourcePanelId && targetPanelId && message.trim()) &&
    messageBytes <= MAX_MESSAGE_BYTES;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>다른 에이전트 런으로 메시지 보내기</DialogTitle>
          <DialogDescription>
            같은 Worktree Session 창에 열린 패널만 대상으로 선택할 수 있습니다.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="agent-peer-target">대상 패널</Label>
            <Select value={targetPanelId} onValueChange={setTargetPanelId}>
              <SelectTrigger id="agent-peer-target" className="w-full">
                <SelectValue placeholder="대상 선택" />
              </SelectTrigger>
              <SelectContent>
                {peers.map((peer) => (
                  <SelectItem key={peer.panelId} value={peer.panelId}>
                    {peer.title} · {peer.status === "running" ? "실행 중" : "대기"}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="agent-peer-delivery">전달 방식</Label>
            <Select
              value={delivery}
              onValueChange={(value) => setDelivery(value as AgentPromptDelivery)}
            >
              <SelectTrigger id="agent-peer-delivery" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="send">즉시 전송</SelectItem>
                <SelectItem value="queue">현재 작업 이후 대기</SelectItem>
                <SelectItem value="draft">입력 초안으로 준비</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="agent-peer-message">메시지</Label>
            <Textarea
              id="agent-peer-message"
              value={message}
              onChange={(event) => setMessage(event.target.value)}
              rows={8}
              placeholder="대상 에이전트에게 전달할 작업이나 컨텍스트를 입력하세요."
            />
            <p className="text-right text-xs text-muted-foreground">
              {messageBytes.toLocaleString()} / 16,384 bytes
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            취소
          </Button>
          <Button
            type="button"
            disabled={!valid || isSubmitting}
            onClick={async () => {
              if (!sourcePanelId || !valid) {
                return;
              }
              setIsSubmitting(true);
              try {
                await onSubmit({
                  sourcePanelId,
                  targetPanelId,
                  message: message.trim(),
                  delivery,
                });
                setMessage("");
                onOpenChange(false);
              } finally {
                setIsSubmitting(false);
              }
            }}
          >
            메시지 전달
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
