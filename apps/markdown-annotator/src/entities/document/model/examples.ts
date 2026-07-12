import agentReviewPlan from "@examples/agent-review-plan.md?raw";
import mermaidChartExamples from "@examples/mermaid-chart-examples.md?raw";
import personalNotesShoppingList from "@examples/personal-notes-shopping-list.md?raw";
import productRequirements from "@examples/product-requirements.md?raw";
import speckitChecklist from "@examples/speckit-checklist.md?raw";
import speckitDataModel from "@examples/speckit-data-model.md?raw";
import speckitPlan from "@examples/speckit-plan.md?raw";
import speckitSpec from "@examples/speckit-spec.md?raw";
import speckitTasks from "@examples/speckit-tasks.md?raw";
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
    id: "mermaid-chart-examples",
    fileName: "mermaid-chart-examples.md",
    title: "Mermaid Chart Examples",
    description: "Mermaid flowchart, sequence, state diagram 렌더링을 확인하는 예제입니다.",
    markdownText: mermaidChartExamples,
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
    id: "speckit-spec",
    fileName: "speckit-spec.md",
    title: "SpecKit · Feature Specification",
    description: "사용자 시나리오, 요구사항, 성공 기준으로 구성된 spec.md 예제입니다.",
    markdownText: speckitSpec,
  },
  {
    id: "speckit-plan",
    fileName: "speckit-plan.md",
    title: "SpecKit · Implementation Plan",
    description: "기술 컨텍스트, 구조, constitution 검사를 포함한 plan.md 예제입니다.",
    markdownText: speckitPlan,
  },
  {
    id: "speckit-data-model",
    fileName: "speckit-data-model.md",
    title: "SpecKit · Data Model",
    description: "엔티티와 상태 전이를 설명하는 data-model.md 예제입니다.",
    markdownText: speckitDataModel,
  },
  {
    id: "speckit-tasks",
    fileName: "speckit-tasks.md",
    title: "SpecKit · Tasks",
    description: "단계별 의존성과 완료 상태를 포함한 tasks.md 예제입니다.",
    markdownText: speckitTasks,
  },
  {
    id: "speckit-checklist",
    fileName: "speckit-checklist.md",
    title: "SpecKit · Requirements Checklist",
    description: "명세 품질을 검토하는 requirements checklist 예제입니다.",
    markdownText: speckitChecklist,
  },
  {
    id: "technical-design",
    fileName: "technical-design.md",
    title: "Technical Design",
    description: "Stable node id와 offset anchor 설계를 설명합니다.",
    markdownText: technicalDesign,
  },
];
