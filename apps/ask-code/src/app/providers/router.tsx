import { createHashRouter, RouterProvider } from "react-router";

import { AskCodePage } from "@/pages/ask-code";

const router = createHashRouter([
  {
    path: "/",
    element: <AskCodePage />,
  },
]);

export function AppRouter() {
  return <RouterProvider router={router} />;
}
