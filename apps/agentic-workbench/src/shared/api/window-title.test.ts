// 044 T029·T032: 창 제목은 데스크톱 표현 command로 적용한다(외부 서버는 창에 제목을 넣지 않는다, research R3).
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn(async () => undefined);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...(args as [])) }));

import { applyWindowTitle } from "./window-title";

describe("applyWindowTitle", () => {
  beforeEach(() => invoke.mockClear());

  it("asks the desktop to apply the title to the window and its native menu", async () => {
    await applyWindowTitle("repo · feature");
    expect(invoke).toHaveBeenCalledWith("apply_window_title", { title: "repo · feature" });
  });
});
