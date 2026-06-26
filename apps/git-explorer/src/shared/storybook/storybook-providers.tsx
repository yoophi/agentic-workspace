import type { PropsWithChildren } from "react";

import { QueryProvider } from "@/app/providers/query";

export function StorybookProviders({ children }: PropsWithChildren) {
  return <QueryProvider>{children}</QueryProvider>;
}
