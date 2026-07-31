import { useEffect, useId, useState } from "react";

import { Slider } from "@/components/ui/slider";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from "@/components/ui/field";
import {
  FONT_SIZE_STEPS,
  formatFontSizeStep,
  normalizeFontSizeStep,
} from "@/entities/appearance-preferences/model/font-size-step";
import type { FontSizeStep } from "@/entities/appearance-preferences/model/types";

export type FontSizeSliderProps = {
  value: FontSizeStep;
  onValueChange: (value: FontSizeStep) => Promise<unknown> | unknown;
  disabled?: boolean;
  isLoading?: boolean;
  error?: string | null;
};

export function FontSizeSlider({
  value,
  onValueChange,
  disabled = false,
  isLoading = false,
  error: externalError = null,
}: FontSizeSliderProps) {
  const id = useId();
  const [draft, setDraft] = useState<FontSizeStep>(value);
  const [isPending, setPending] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (!isPending) {
      setDraft(value);
    }
  }, [isPending, value]);

  async function change(nextValues: number[]) {
    if (isPending || nextValues[0] === undefined) {
      return;
    }
    const next = normalizeFontSizeStep(nextValues[0]);
    const previous = value;
    setDraft(next);
    setSaveError(null);
    setPending(true);
    try {
      await onValueChange(next);
    } catch (cause) {
      setDraft(previous);
      setSaveError(String(cause));
    } finally {
      setPending(false);
    }
  }

  const error = saveError ?? externalError;
  const unavailable = disabled || isLoading || isPending;

  return (
    <Field data-invalid={Boolean(error)}>
      <div className="flex items-center justify-between gap-3">
        <FieldLabel htmlFor={id}>글꼴 크기</FieldLabel>
        <output
          className="min-w-8 rounded-md border bg-muted px-2 py-0.5 text-center text-sm tabular-nums"
          htmlFor={id}
        >
          {formatFontSizeStep(draft)}
        </output>
      </div>
      <Slider
        id={id}
        aria-label="글꼴 크기"
        aria-valuetext={`${formatFontSizeStep(draft)} 단계`}
        min={FONT_SIZE_STEPS[0]}
        max={FONT_SIZE_STEPS[FONT_SIZE_STEPS.length - 1]}
        step={1}
        value={[draft]}
        disabled={unavailable}
        onValueChange={(next) => void change(next)}
      />
      <div
        aria-hidden="true"
        className="grid grid-cols-5 text-center text-xs text-muted-foreground"
      >
        {FONT_SIZE_STEPS.map((step) => (
          <span key={step}>{formatFontSizeStep(step)}</span>
        ))}
      </div>
      <FieldDescription>
        창의 텍스트만 단계별로 조정합니다. 아이콘과 이미지 크기는 유지됩니다.
        {isLoading && " 현재 설정을 불러오는 중입니다."}
        {isPending && " 저장하는 중입니다."}
      </FieldDescription>
      <FieldError>{error}</FieldError>
    </Field>
  );
}
