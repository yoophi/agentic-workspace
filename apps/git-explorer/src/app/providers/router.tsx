import { createHashRouter, RouterProvider } from "react-router";
import { RepositoryPage } from "@/pages/repository";

const router = createHashRouter([
  {
    path: "/",
    element: <RepositoryPage />,
  },
]);

export function AppRouter() {
  return <RouterProvider router={router} />;
}
