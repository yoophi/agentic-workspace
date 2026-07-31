import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { FontSizeSlider } from "./font-size-slider";

describe("FontSizeSlider", () => {
  it("renders a single controlled five-step thumb with accessible value text", () => {
    const html = renderToStaticMarkup(
      <FontSizeSlider value={2} onValueChange={vi.fn()} />,
    );

    expect(html).toContain("글꼴 크기");
    expect(html).toContain("aria-valuemin=\"-2\"");
    expect(html).toContain("aria-valuemax=\"2\"");
    expect(html).toContain("aria-valuenow=\"2\"");
    expect(html).toContain("aria-valuetext=\"+2 단계\"");
    expect((html.match(/data-slot="slider-thumb"/g) ?? [])).toHaveLength(1);
    for (const label of ["-2", "-1", "0", "+1", "+2"]) {
      expect(html).toContain(label);
    }
  });

  it("keeps its structure and exposes loading and error states", () => {
    const html = renderToStaticMarkup(
      <FontSizeSlider
        value={0}
        onValueChange={vi.fn()}
        isLoading
        error="저장 실패"
      />,
    );
    expect(html).toContain("현재 설정을 불러오는 중입니다.");
    expect(html).toContain("저장 실패");
    expect(html).toContain("role=\"alert\"");
    expect(html).toContain("data-disabled");
  });
});
