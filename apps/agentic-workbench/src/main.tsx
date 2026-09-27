import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import { App } from "./app/App";
import { QueryProvider } from "./app/providers/query-provider";
import { AppearancePreferencesProvider } from "./app/providers/appearance-preferences-provider";
import { bootstrapTransport } from "./app/bootstrap-transport";
import { ConnectionFailure } from "./shared/ui/connection-failure";
import { ConnectionStatus } from "./shared/ui/connection-status";
import "./index.css";

if (import.meta.env.DEV) {
  void import("react-grab");
}

// 043: 창이 뜰 때 경로를 한 번 정한다(네트워크 경로 또는 호환 경로, research R3). 정하기 전에는 화면을 그리지 않는다 —
// 첫 호출·구독부터 한 경로를 쓰게 하기 위해서다. 044: 외부 서버 모드에서 연결에 실패하면 연결 실패 화면만 그린다 — 다시
// 시도는 창을 다시 불러와 처음부터 부팅한다.
void bootstrapTransport().then(
  (result) => render(result.kind === "failed" ? (result.reason ?? "unknown error") : null),
  (error: unknown) => render(error instanceof Error ? error.message : String(error)),
);

function render(failure: string | null) {
  const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
  if (failure !== null) {
    root.render(
      <React.StrictMode>
        <ConnectionFailure reason={failure} onRetry={() => window.location.reload()} />
      </React.StrictMode>,
    );
    return;
  }
  root.render(
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
}
