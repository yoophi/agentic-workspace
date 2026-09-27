// @vitest-environment happy-dom
// 044 T032: 외부 서버 모드에서 연결에 실패한 창은 이유와 다시 시도를 보여 준다(호환 경로 대체 없음).
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CONNECTION_FAILURE_TITLE, ConnectionFailure } from "./connection-failure";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  document.body.innerHTML = "";
});

describe("ConnectionFailure", () => {
  it("shows the reason and retries on request", () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    const onRetry = vi.fn();
    act(() => root.render(<ConnectionFailure reason="Workbench server executable was not found" onRetry={onRetry} />));
    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain(CONNECTION_FAILURE_TITLE);
    expect(alert?.textContent).toContain("Workbench server executable was not found");
    const retry = container.querySelector("button");
    expect(retry?.textContent).toBe("다시 시도");
    act(() => retry?.click());
    expect(onRetry).toHaveBeenCalledTimes(1);
    act(() => root.unmount());
  });
});
