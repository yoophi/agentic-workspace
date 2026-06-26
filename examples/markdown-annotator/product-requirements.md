# Markdown Annotation Product Requirements

## Summary

The product provides a focused workspace for reviewing Markdown documents and turning user annotations into structured feedback.

## User Needs

Reviewers need to point at exact sentences instead of rewriting long instructions manually. Agents need feedback in a compact format that includes the file name, selected text, annotation type, and comment.

## Functional Requirements

1. Users can choose an example document.
2. Users can open a local Markdown file with the Tauri file dialog.
3. Users can select text in the rendered document.
4. Users can add a question, change request, note, or approval annotation.
5. Users can copy an agent prompt generated from all annotations.

## Non-Goals

- Multi-user collaboration
- Image annotation
- URL annotation
- Direct submission to a specific agent protocol

## Acceptance Criteria

The exported prompt must be readable as plain Markdown and must not require the agent to understand browser selection metadata.
