import { AnnotatorPage } from "@/pages/annotator/AnnotatorPage";
import { TooltipProvider } from "@/components/ui/tooltip";

export function App() {
  return (
    <TooltipProvider>
      <AnnotatorPage />
    </TooltipProvider>
  );
}
