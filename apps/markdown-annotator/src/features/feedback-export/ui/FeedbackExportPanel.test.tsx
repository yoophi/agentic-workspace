import { describe, expect, it } from "vitest";
import { defaultSelectedAnnotationIds } from "./FeedbackExportPanel";
import type { ReviewSession } from "@/entities/review-session/model/types";
it("selects open annotations by default",()=>{const session:ReviewSession={sessionId:"s",revision:0,documentPath:"a.md",decision:"draft",annotations:[{id:"a",groupId:null,type:"note",status:"open",comment:"",selectedText:"a",attachmentState:"attached"},{id:"b",groupId:null,type:"note",status:"resolved",comment:"",selectedText:"b",attachmentState:"attached"}]};expect(defaultSelectedAnnotationIds(session)).toEqual(["a"]);});
