import { QueryProvider } from "./providers/query";
import { AppRouter } from "./providers/router";

export function App() {
  return (
    <QueryProvider>
      <AppRouter />
    </QueryProvider>
  );
}
