import { PlusIcon, Trash2Icon } from "lucide-react";

import type { EnvDraftRow } from "@/features/agent-command-override/model/command-override-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type EnvVarEditorProps = {
  rows: EnvDraftRow[];
  onAddRow: () => void;
  onUpdateRow: (rowId: string, key: string, value: string) => void;
  onRemoveRow: (rowId: string) => void;
  addLabel?: string;
};

/**
 * 환경변수 key/value 편집기(specs/008 US1). key가 비어 있거나 공백뿐인 행은
 * 저장 시 제거된다(FR-004) — 편집 중에는 그대로 두어 입력 흐름을 방해하지 않는다.
 */
export function EnvVarEditor({
  rows,
  onAddRow,
  onUpdateRow,
  onRemoveRow,
  addLabel = "환경변수 추가",
}: EnvVarEditorProps) {
  return (
    <div className="flex flex-col gap-2">
      {rows.length === 0 ? (
        <p className="text-xs text-muted-foreground">설정된 환경변수가 없습니다.</p>
      ) : (
        rows.map((row) => (
          <div key={row.id} className="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)_auto] gap-2">
            <Input
              value={row.key}
              placeholder="KEY"
              className="min-w-0 font-mono"
              aria-label="환경변수 이름"
              onChange={(event) => onUpdateRow(row.id, event.target.value, row.value)}
            />
            <Input
              value={row.value}
              placeholder="value"
              className="min-w-0 font-mono"
              aria-label="환경변수 값"
              onChange={(event) => onUpdateRow(row.id, row.key, event.target.value)}
            />
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="환경변수 행 삭제"
              onClick={() => onRemoveRow(row.id)}
            >
              <Trash2Icon />
            </Button>
          </div>
        ))
      )}
      <Button type="button" variant="outline" size="sm" className="w-fit" onClick={onAddRow}>
        <PlusIcon data-icon="inline-start" />
        {addLabel}
      </Button>
    </div>
  );
}
