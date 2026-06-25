import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import { App } from "./app/App";
import { QueryProvider } from "./app/providers/query-provider";
import "./index.css";

if (import.meta.env.DEV) {
  void import("react-grab");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryProvider>
      <HashRouter>
        <App />
      </HashRouter>
    </QueryProvider>
  </React.StrictMode>,
);
