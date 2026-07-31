import { describe, expect, it, vi } from "vitest";

import { schedulePanelResize } from "./use-split-persistence";

describe("schedulePanelResize", () => {
  it("waits for the dynamic panel layout and retries a transient missing-layout error", () => {
    const frames: FrameRequestCallback[] = [];
    const resize = vi
      .fn<(size: number | string) => void>()
      .mockImplementationOnce(() => {
        throw new Error("Layout not found for Panel project-worktree-session-workspace");
      });

    schedulePanelResize(
      () => ({ resize }),
      480,
      {
        requestFrame: (callback) => {
          frames.push(callback);
          return frames.length;
        },
        cancelFrame: vi.fn(),
      },
    );

    expect(resize).not.toHaveBeenCalled();

    frames.shift()?.(0);
    expect(resize).toHaveBeenCalledTimes(1);
    expect(frames).toHaveLength(1);

    frames.shift()?.(16);
    expect(resize).toHaveBeenCalledTimes(2);
    expect(resize).toHaveBeenLastCalledWith(480);
  });

  it("does not hide unrelated resize errors", () => {
    const frames: FrameRequestCallback[] = [];
    const unexpected = new Error("Unexpected resize failure");

    schedulePanelResize(
      () => ({
        resize: () => {
          throw unexpected;
        },
      }),
      480,
      {
        requestFrame: (callback) => {
          frames.push(callback);
          return frames.length;
        },
        cancelFrame: vi.fn(),
      },
    );

    expect(() => frames.shift()?.(0)).toThrow(unexpected);
  });
});
