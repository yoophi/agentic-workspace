import { AnnotatorPage } from "@/pages/annotator/AnnotatorPage";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ApplicationEventsProvider } from "@/app/providers/application-events-provider";
import { SettingsPage } from "@/pages/settings/SettingsPage";
import { AboutPage } from "@/pages/about/AboutPage";
import { StartPage } from "@/pages/start/StartPage";

export function App() {
  const page = new URL(window.location.href).searchParams.get("page");
  const hasLaunchTarget = new URL(window.location.href).searchParams.has("root") || new URL(window.location.href).searchParams.has("path");
  return (
    <ApplicationEventsProvider>
      <TooltipProvider>
        {page === "settings" ? <SettingsPage /> : page === "about" ? <AboutPage /> : hasLaunchTarget ? <AnnotatorPage /> : <StartPage />}
      </TooltipProvider>
    </ApplicationEventsProvider>
  );
}
