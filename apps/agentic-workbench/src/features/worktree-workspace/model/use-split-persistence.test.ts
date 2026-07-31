import { describe, expect, it, vi } from "vitest";

import { schedulePanelResize } from "./use-split-persistence";

describe("schedulePanelResize", () => {
  it.each([
    "Group project-worktree-session-group not found",
    "Panel constraints not found for Panel project-worktree-session-workspace",
    "Layout not found for Panel project-worktree-session-workspace",
  ])("waits for the dynamic panel layout and retries %s", (message) => {
    const frames: FrameRequestCallback[] = [];
    const resize = vi
      .fn<(size: number | string) => void>()
      .mockImplementationOnce(() => {
        throw new Error(message);
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

  it("keeps the rendered screen alive when restoring a saved width fails", () => {
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

    expect(() => frames.shift()?.(0)).not.toThrow();
    expect(frames).toHaveLength(0);
  });

  it("stops retrying after the bounded attempt count without throwing", () => {
    const frames: FrameRequestCallback[] = [];
    const resize = vi.fn(() => {
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

    expect(() => {
      while (frames.length > 0) frames.shift()?.(0);
    }).not.toThrow();
    expect(resize).toHaveBeenCalledTimes(3);
  });

  it("cancels a pending resize when its split leaves the screen", () => {
    const frames: FrameRequestCallback[] = [];
    const cancelFrame = vi.fn();
    const resize = vi.fn();

    const cancel = schedulePanelResize(
      () => ({ resize }),
      480,
      {
        requestFrame: (callback) => {
          frames.push(callback);
          return frames.length;
        },
        cancelFrame,
      },
    );

    cancel();
    frames.shift()?.(0);

    expect(cancelFrame).toHaveBeenCalledWith(1);
    expect(resize).not.toHaveBeenCalled();
  });
});
