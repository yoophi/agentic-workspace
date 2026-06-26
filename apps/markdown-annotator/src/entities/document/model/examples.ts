import agentReviewPlan from "@examples/agent-review-plan.md?raw";
import personalNotesShoppingList from "@examples/personal-notes-shopping-list.md?raw";
import productRequirements from "@examples/product-requirements.md?raw";
import technicalDesign from "@examples/technical-design.md?raw";

export type ExampleMarkdownDocument = {
  id: string;
  fileName: string;
  title: string;
  description: string;
  markdownText: string;
};

export const exampleMarkdownDocuments: ExampleMarkdownDocument[] = [
  {
    id: "agent-review-plan",
    fileName: "agent-review-plan.md",
    title: "Agent Review Workflow Plan",
    description: "Agent가 생성한 계획을 사람이 검토하는 흐름의 예제입니다.",
    markdownText: agentReviewPlan,
  },
  {
    id: "personal-notes-shopping-list",
    fileName: "personal-notes-shopping-list.md",
    title: "Personal Notes and Shopping List",
    description: "간단한 메모와 장보기 목록을 포함한 테스트 문서입니다.",
    markdownText: personalNotesShoppingList,
  },
  {
    id: "product-requirements",
    fileName: "product-requirements.md",
    title: "Product Requirements",
    description: "Annotation MVP 요구사항을 정리한 문서입니다.",
    markdownText: productRequirements,
  },
  {
    id: "technical-design",
    fileName: "technical-design.md",
    title: "Technical Design",
    description: "Stable node id와 offset anchor 설계를 설명합니다.",
    markdownText: technicalDesign,
  },
];
