import { QueryProvider } from "@/app/providers/query";
import { AppRouter } from "@/app/providers/router";

export function App() {
  return (
    <QueryProvider>
      <AppRouter />
    </QueryProvider>
  );
}
