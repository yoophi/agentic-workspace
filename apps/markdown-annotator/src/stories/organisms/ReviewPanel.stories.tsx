import type { Meta, StoryObj } from "@storybook/react-vite";
import { ReviewDecisionPanel } from "@/features/review-session/ui/ReviewDecisionPanel";
import { useReviewSessionStore } from "@/features/review-session/model/review-session-store";

const meta = { title: "Organisms/Review Panel", component: ReviewDecisionPanel, decorators: [(Story: typeof ReviewDecisionPanel) => { useReviewSessionStore.setState({ session: { sessionId: "story", revision: 1, documentPath: "guide.md", decision: "changes-requested", annotations: [{ id: "a", groupId: null, type: "change-request", status: "open", comment: "표현을 명확하게 바꿔주세요.", selectedText: "모호한 문장", attachmentState: "attached" }] }, warning: null }); return <Story />; }] } satisfies Meta<typeof ReviewDecisionPanel>;
export default meta;
export const ChangesRequested: StoryObj<typeof meta> = {};
