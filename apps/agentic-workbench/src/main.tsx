import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import { App } from "./app/App";
import { QueryProvider } from "./app/providers/query-provider";
import { AppearancePreferencesProvider } from "./app/providers/appearance-preferences-provider";
import { bootstrapTransport } from "./app/bootstrap-transport";
import { ConnectionStatus } from "./shared/ui/connection-status";
import "./index.css";

if (import.meta.env.DEV) {
  void import("react-grab");
}

// 043: 창이 뜰 때 경로를 한 번 정한다(네트워크 경로 또는 호환 경로, research R3). 정하기 전에는 화면을 그리지 않는다 —
// 첫 호출·구독부터 한 경로를 쓰게 하기 위해서다.
void bootstrapTransport().finally(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <QueryProvider>
        <AppearancePreferencesProvider>
          <HashRouter>
            <App />
          </HashRouter>
          <ConnectionStatus />
        </AppearancePreferencesProvider>
      </QueryProvider>
    </React.StrictMode>,
  );
});
