// @vitest-environment happy-dom
// 043 T048: 연결 상태 표시는 끊겼을 때(재연결 중·연결 불가)만 보이고, 연결되면 사라진다.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";

import { setConnectionStatus } from "@/shared/api/transport/connection-status";

import { CONNECTION_STATUS_MESSAGES, ConnectionStatus } from "./connection-status";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  act(() => setConnectionStatus("connected"));
  document.body.innerHTML = "";
});

describe("ConnectionStatus", () => {
  it("shows only while reconnecting or disconnected", () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    act(() => root.render(<ConnectionStatus />));
    expect(container.textContent).toBe("");
    act(() => setConnectionStatus("reconnecting"));
    expect(container.textContent).toBe(CONNECTION_STATUS_MESSAGES.reconnecting);
    act(() => setConnectionStatus("disconnected"));
    expect(container.textContent).toBe(CONNECTION_STATUS_MESSAGES.disconnected);
    act(() => setConnectionStatus("connected"));
    expect(container.textContent).toBe("");
    act(() => root.unmount());
  });
});
