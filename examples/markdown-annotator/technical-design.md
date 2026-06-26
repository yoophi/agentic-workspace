# Technical Design: Stable Markdown Annotation

## Principle

Annotations should attach to the document model, not to incidental DOM structure.

## Proposed Data Model

```ts
type AnnotationAnchor = {
  blockId: string
  startOffset?: number
  endOffset?: number
  selectedText?: string
  startLine?: number
  endLine?: number
}
```

The `blockId` identifies a rendered Markdown block. Offsets describe the selected text inside that block.

## Rendering

Each meaningful Markdown element should receive a stable id.

```html
<p data-block-id="block-12" data-start-line="18" data-end-line="18">
  Use filesystem JSON cache for the MVP.
</p>
```

## Export

Feedback should be formatted for agent consumption.

```markdown
# Markdown Annotations

File: docs/plan.md

## 1. [change-request] Feedback on: "Use SQLite"
> Prefer a filesystem JSON cache for the MVP.
```

## Tradeoff

The first version can use simple block-level anchors. A later version can improve precision with Markdown AST positions and robust text matching.
