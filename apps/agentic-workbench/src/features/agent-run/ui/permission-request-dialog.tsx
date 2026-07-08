import { useEffect, useRef, useState } from "react";

import {
  createPermissionDisplayModel,
  type PermissionEvent,
} from "@/entities/agent-run/model";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

type PermissionRequestDialogProps = {
  permission: PermissionEvent | null;
  onSelect: (permissionId: string, optionId: string) => Promise<void>;
};

type PermissionRequestDialogFrameProps = PermissionRequestDialogProps & {
  submittingOptionId?: string | null;
  isSubmitting?: boolean;
};

export function PermissionRequestDialog({ permission, onSelect }: PermissionRequestDialogProps) {
  const permissionId = permission?.permissionId;
  const [submittingOptionId, setSubmittingOptionId] = useState<string | null>(null);
  const submittingPermissionIdRef = useRef<string | null>(null);
  const isSubmitting =
    submittingPermissionIdRef.current === permissionId && submittingOptionId !== null;

  useEffect(() => {
    submittingPermissionIdRef.current = null;
    setSubmittingOptionId(null);
  }, [permissionId]);

  async function submitPermission(nextPermissionId: string, optionId: string) {
    if (submittingPermissionIdRef.current === nextPermissionId) {
      return;
    }

    submittingPermissionIdRef.current = nextPermissionId;
    setSubmittingOptionId(optionId);

    try {
      await onSelect(nextPermissionId, optionId);
    } catch {
      submittingPermissionIdRef.current = null;
      setSubmittingOptionId(null);
    }
  }

  return (
    <Dialog open={Boolean(permissionId)}>
      <DialogContent
        showCloseButton={false}
        className="grid max-h-[calc(100dvh-2rem)] w-[min(calc(100vw-1rem),44rem)] grid-rows-[auto_minmax(0,1fr)_auto] gap-0 overflow-hidden p-0 sm:max-w-2xl"
      >
        <PermissionRequestDialogFrame
          permission={permission}
          onSelect={submitPermission}
          submittingOptionId={submittingOptionId}
          isSubmitting={isSubmitting}
        />
      </DialogContent>
    </Dialog>
  );
}

export function PermissionRequestDialogFrame({
  permission,
  onSelect,
  submittingOptionId = null,
  isSubmitting = false,
}: PermissionRequestDialogFrameProps) {
  const model = createPermissionDisplayModel(permission);
  const canSubmit = Boolean(model.permissionId);

  return (
    <>
      <DialogHeader className="border-b px-4 py-4 sm:px-5">
        <DialogTitle>Permission required</DialogTitle>
        <DialogDescription>
          Agent가 작업을 계속하려면 아래 요청에 대한 결정을 선택해야 합니다.
        </DialogDescription>
      </DialogHeader>

      <div className="min-h-0 overflow-y-auto px-4 py-4 sm:px-5" data-testid="permission-body">
        <section className="grid gap-3">
          <div className="rounded-lg border bg-muted/35 p-3">
            <div className="text-sm font-medium break-words">{model.title}</div>
            <p className="mt-1 text-xs text-muted-foreground">
              원문 요청과 승인 범위를 확인한 뒤 선택하세요.
            </p>
          </div>

          {model.detail && (
            <div className="grid gap-2">
              <div className="text-xs font-medium text-muted-foreground">
                {model.detail.label}
              </div>
              <pre
                tabIndex={0}
                data-testid="permission-detail"
                data-long={model.detail.isLong ? "true" : "false"}
                className="max-h-[min(42dvh,26rem)] overflow-auto whitespace-pre-wrap break-all rounded-lg border bg-background p-3 font-mono text-[11px] leading-relaxed text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 sm:text-xs"
              >
                {model.detail.text}
              </pre>
            </div>
          )}

          {model.options.some((option) => option.fullLabel !== option.buttonLabel) && (
            <div className="rounded-lg border bg-background p-3" data-testid="permission-option-detail">
              <div className="text-xs font-medium text-muted-foreground">Approval options</div>
              <dl className="mt-2 grid gap-2 text-xs">
                {model.options.map((option) => (
                  <div key={option.optionId} className="grid gap-1">
                    <dt className="font-medium">{option.buttonLabel}</dt>
                    <dd className="break-all font-mono text-muted-foreground">{option.fullLabel}</dd>
                  </div>
                ))}
              </dl>
            </div>
          )}
        </section>
      </div>

      <DialogFooter
        className="m-0 flex-col-reverse items-stretch rounded-none border-t bg-muted/45 p-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end sm:px-5"
        data-testid="permission-actions"
      >
        {model.options.map((option) => (
          <Button
            key={option.optionId}
            type="button"
            variant={option.isDestructiveOrReject ? "outline" : "default"}
            disabled={isSubmitting || !canSubmit}
            aria-label={`${option.buttonLabel}: ${option.fullLabel}`}
            data-option-id={option.optionId}
            className="min-w-0 justify-center"
            onClick={() => {
              if (model.permissionId) {
                void onSelect(model.permissionId, option.optionId);
              }
            }}
          >
            <span className="truncate">
              {submittingOptionId === option.optionId ? "Submitting" : option.buttonLabel}
            </span>
          </Button>
        ))}
      </DialogFooter>
    </>
  );
}
