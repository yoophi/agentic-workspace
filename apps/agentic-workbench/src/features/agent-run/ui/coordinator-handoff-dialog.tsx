import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";

type CoordinatorHandoffDialogProps = {
  open: boolean;
  previousRunId: string;
  successorRunId: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: (summary: string) => void | Promise<void>;
};

export function CoordinatorHandoffDialog({
  open,
  previousRunId,
  successorRunId,
  onOpenChange,
  onConfirm,
}: CoordinatorHandoffDialogProps) {
  const [summary, setSummary] = useState("");
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Main Coordinator 인계</DialogTitle>
          <DialogDescription>
            진행 중인 task ownership을 {previousRunId}에서 {successorRunId}로 옮깁니다.
            하위 agent-run은 재시작되지 않습니다.
          </DialogDescription>
        </DialogHeader>
        <Textarea
          value={summary}
          onChange={(event) => setSummary(event.target.value)}
          aria-label="Coordinator 인계 요약"
          placeholder="새 Main이 알아야 할 진행 상황과 미해결 항목"
        />
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            취소
          </Button>
          <Button
            type="button"
            disabled={!summary.trim()}
            onClick={() => void onConfirm(summary.trim())}
          >
            확인 후 인계
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

