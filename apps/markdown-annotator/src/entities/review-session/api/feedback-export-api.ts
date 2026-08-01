import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { ReviewSession } from "../model/types";
import { toReviewSessionWire } from "./review-session-api";
export type FeedbackExport = { json: string; markdown: string };
export const exportFeedback = (session: ReviewSession, includeResolved: boolean, selectedIds: string[]) => invoke<FeedbackExport>("export_review_feedback", { session: toReviewSessionWire(session), exportedAt: new Date().toISOString(), includeResolved, selectedIds });
export async function copyFeedback(content: string) { if (!navigator.clipboard) throw new Error("CLIPBOARD_UNAVAILABLE"); await navigator.clipboard.writeText(content); }
export async function saveFeedback(content: string, extension: "md" | "json") { const path = await save({ defaultPath: `review-feedback.${extension}` }); if (!path) return false; await invoke("save_feedback_file", { path, content }); return true; }
