import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Button } from "./button";

describe("Button", () => {
  it("keeps the foreground color on small default buttons", () => {
    const html = renderToStaticMarkup(
      <Button variant="default" size="sm">
        포커스
      </Button>,
    );

    expect(html).toContain("bg-primary");
    expect(html).toContain("text-primary-foreground");
    expect(html).toContain("text-[length:var(--text-compact)]");
  });
});
