# Agent Review Workflow Plan

## Goal

Build a small desktop review tool that lets a human mark up an agent-generated Markdown plan before the agent continues.

## Current Proposal

The application should store every annotation directly on the rendered DOM node and serialize the browser selection object when exporting feedback.

This approach is fast to prototype, but it couples annotation state to React's current render output. If Markdown parsing changes or a heading is inserted above the target paragraph, restored selections may point to the wrong text.

## MVP

- Open a Markdown file from disk
- Render Markdown with tables, lists, headings, and code blocks
- Let the user select text and write comments
- Export the comments as an agent prompt

## Open Question

Should the first version submit feedback directly to Codex, or should it only copy a Markdown prompt to the clipboard?

## Risk

The plan does not define a stable anchor format. Without stable anchors, annotations will be difficult to preserve across re-renders.
