import type {
  ButtonHTMLAttributes,
  ComponentType,
  ReactElement,
  ReactNode,
  TextareaHTMLAttributes,
} from "react";
import type { AnnotationType } from "@yoophi/markdown-annotation-core/types";
import type { MarkdownBlock } from "@yoophi/markdown-annotation-core/types";

/**
 * Viewer 전용 inline annotation 표현. core의 AnnotationDraft에서 viewer 렌더에
 * 필요한 필드만 평탄화한 형태다(렌더 텍스트 offset 기준).
 */
export type MarkdownViewerInlineAnnotation = {
  id: string;
  comment: string;
  endOffset: number;
  startOffset: number;
  type: AnnotationType;
};

export type MarkdownViewerBlockNote = {
  id: string;
  comment: string;
};

/**
 * 주입되는 Button. MA(base-ui)·AW(radix) 양쪽 shadcn Button이 그대로 만족하는
 * 최소 계약이다(variant/size는 두 앱이 동일한 cva 토큰을 공유).
 */
export type ViewerButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "outline" | "secondary" | "ghost" | "destructive" | "link";
  size?: "default" | "xs" | "sm" | "lg" | "icon" | "icon-xs" | "icon-sm" | "icon-lg";
};

/**
 * 주입되는 Tooltip. trigger(children)와 content를 받는 통합 계약으로,
 * MA는 base-ui(`render`), AW는 radix(`asChild`) 어댑터로 각각 구현한다.
 * Tooltip 합성 API가 키트마다 다르므로 viewer는 이 계약에만 의존한다.
 */
export type ViewerTooltipProps = {
  content: ReactNode;
  align?: "start" | "center" | "end";
  children: ReactElement;
};

export type MermaidExpandedDialogRootProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
};

export type MermaidExpandedDialogTriggerProps = {
  children: ReactElement;
  tooltip: ReactNode;
};

export type MermaidExpandedDialogContentProps = {
  title: ReactNode;
  description: ReactNode;
  children: ReactNode;
};

export type MermaidExpandedDialogComponents = {
  Root: ComponentType<MermaidExpandedDialogRootProps>;
  Trigger: ComponentType<MermaidExpandedDialogTriggerProps>;
  Content: ComponentType<MermaidExpandedDialogContentProps>;
};

/**
 * 키트 비종속화를 위해 주입하는 UI primitive 모음.
 */
export type MarkdownViewerComponents = {
  Button: ComponentType<ViewerButtonProps>;
  Tooltip: ComponentType<ViewerTooltipProps>;
  MermaidExpandedDialog?: MermaidExpandedDialogComponents;
};

export type MarkdownViewerBlockQuickPromptAction = {
  accessibleName?: string;
  tooltip?: ReactNode;
  disabled?: boolean;
  disabledReason?: ReactNode;
  onRequest: (block: MarkdownBlock) => void;
};

export type TypeSelectOption = {
  value: AnnotationType;
  label: string;
};

/**
 * 주입되는 Dialog 셸. Dialog 합성 API가 키트마다 다르므로(base-ui `render`,
 * radix `asChild`), title/description/footer/children을 받는 통합 계약으로 흡수한다.
 */
export type DialogShellProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  footer: ReactNode;
  children: ReactNode;
};

/**
 * 주입되는 타입 Select. value/options만 받는 통합 계약으로, 각 앱이 자기 Select
 * 합성(base-ui/radix)으로 구현한다.
 */
export type TypeSelectProps = {
  value: AnnotationType;
  onValueChange: (value: AnnotationType) => void;
  options: TypeSelectOption[];
  ariaLabel?: string;
};

/**
 * AnnotationInputDialog에 주입하는 UI primitive 모음.
 * Textarea/Button은 두 앱의 소비 API가 동일하므로 그대로 주입한다.
 */
export type AnnotationDialogComponents = {
  DialogShell: ComponentType<DialogShellProps>;
  TypeSelect: ComponentType<TypeSelectProps>;
  Textarea: ComponentType<TextareaHTMLAttributes<HTMLTextAreaElement>>;
  Button: ComponentType<ViewerButtonProps>;
};
