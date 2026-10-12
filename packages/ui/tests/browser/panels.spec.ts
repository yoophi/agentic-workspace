import { expect, test, type Page } from "@playwright/test";

async function openDemo(page: Page, name: string) {
  await page.goto(`/iframe.html?id=atomic-design-molecules-registered-components--${name}&viewMode=story`);
  await expect(page.getByRole("button", { name: "Chat", exact: true })).toHaveAttribute("aria-expanded", "false");
}
async function panelWidth(page: Page, id: string) {
  return page.getByTestId(id).evaluate((element) => element.getBoundingClientRect().width);
}
async function rememberedRight(page: Page) {
  return page.getByLabel("Panel state").evaluate((el) => JSON.parse(el.textContent!).rightSize as number);
}

for (const [name, expected] of [
  ["persistent-panels-percentage", 320],
  ["persistent-panels-pixels", 320],
  ["persistent-panels-numeric-pixels", 320],
  ["persistent-panels-constrained", 420],
] as const) {
  test(`${name}: measured initial collapse restores its real width`, async ({ page }) => {
    await openDemo(page, name);
    expect(await panelWidth(page, "persistent-chat")).toBeCloseTo(40, 0);
    // 1000px outer group minus borders and 1px separator.
    const width = expected === 320 && name.endsWith("percentage") ? 997 * .32 : expected;
    expect(await rememberedRight(page)).toBeCloseTo(width / 997 * 100, 0);
    await page.getByRole("button", { name: "Expand chat", exact: true }).click();
    await expect.poll(() => panelWidth(page, "persistent-chat")).toBeCloseTo(width, 0);
    await page.getByRole("button", { name: "Toggle chat", exact: true }).click();
    await page.getByRole("button", { name: "Chat", exact: true }).click();
    await expect.poll(() => panelWidth(page, "persistent-chat")).toBeCloseTo(width, 0);
  });
}

test("same DOM, React drafts, selection and undo survive external/title toggles; hidden content cannot receive focus", async ({ page }) => {
  await openDemo(page, "persistent-panels-pixels");
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  const chat = page.getByRole("textbox", { name: "Chat draft", exact: true });
  const doc = page.getByRole("textbox", { name: "Document draft", exact: true });
  await chat.fill("chat draft"); await chat.press("End"); await chat.pressSequentially("!");
  await chat.evaluate((el: HTMLTextAreaElement) => el.setSelectionRange(2, 5));
  await doc.fill("unsaved document");
  await doc.press("End"); await doc.pressSequentially("!");
  const chatNode = await chat.elementHandle(); const docNode = await doc.elementHandle();
  await page.getByRole("button", { name: "Toggle chat", exact: true }).click();
  await expect(chat).toBeHidden();
  expect(await chatNode!.evaluate((el) => { (el as HTMLElement).focus(); return document.activeElement === el; })).toBe(false);
  expect(await chatNode!.evaluate((el) => el.parentElement!.parentElement!.parentElement!.hasAttribute("inert"))).toBe(true);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  expect(await chatNode!.evaluate((el) => el === document.querySelector('[aria-label="Chat draft"]'))).toBe(true);
  expect(await docNode!.evaluate((el) => el === document.querySelector('[aria-label="Document draft"]'))).toBe(true);
  await expect(chat).toHaveValue("chat draft!");
  expect(await chat.evaluate((el: HTMLTextAreaElement) => [el.selectionStart, el.selectionEnd])).toEqual([2, 5]);
  await expect(doc).toHaveValue("unsaved document!");
  await doc.focus(); await doc.press("ControlOrMeta+z"); await expect(doc).toHaveValue("unsaved document");
  await expect(page.getByLabel("Chat draft mounts")).toHaveText("1");
  await expect(page.getByLabel("Document draft mounts")).toHaveText("1");
});

test("keyboard/pointer resize remembers final width, and narrow container respects pixel min/max", async ({ page }) => {
  await openDemo(page, "persistent-panels-pixels");
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  const separator = page.getByRole("separator");
  const before = await panelWidth(page, "persistent-chat");
  await separator.focus(); await separator.press("ArrowLeft");
  await expect.poll(() => panelWidth(page, "persistent-chat")).toBeGreaterThan(before);
  const box = (await separator.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down(); await page.mouse.move(box.x - 40, box.y + box.height / 2, { steps: 5 }); await page.mouse.up();
  const resized = await panelWidth(page, "persistent-chat");
  expect(resized).toBeGreaterThan(before + 20);
  await page.getByRole("button", { name: "Toggle chat", exact: true }).click();
  await expect(separator).toHaveAttribute("aria-disabled", "true");
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  await expect.poll(() => panelWidth(page, "persistent-chat")).toBeCloseTo(resized, 0);
  await page.getByRole("button", { name: "Toggle chat", exact: true }).click();
  await page.getByRole("button", { name: "Resize container", exact: true }).click();
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  await expect.poll(() => panelWidth(page, "persistent-chat")).toBeGreaterThanOrEqual(159);
  expect(await panelWidth(page, "persistent-chat")).toBeLessThanOrEqual(298);
  expect(await panelWidth(page, "persistent-document")).toBeGreaterThanOrEqual(179);
});

test("last open panel protection and updated restrictions apply to controller and titles", async ({ page }) => {
  await openDemo(page, "persistent-panels-percentage");
  await page.getByRole("button", { name: "Collapse document", exact: true }).click();
  await expect(page.getByRole("button", { name: "Document", exact: true })).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("button", { name: "Document", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  await page.getByRole("button", { name: "Lock document", exact: true }).click();
  await page.getByRole("button", { name: "Collapse document", exact: true }).click();
  await expect(page.getByRole("button", { name: "Document", exact: true })).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("button", { name: "Lock document", exact: true }).click();
  await page.getByRole("button", { name: "Collapse document", exact: true }).click();
  await expect(page.getByRole("button", { name: "Document", exact: true })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("button", { name: "Chat", exact: true })).toBeDisabled();
});


test("existing default consumers still unmount on collapse and showTitle false stays open", async ({ page }) => {
  await page.goto("/iframe.html?id=atomic-design-molecules-registered-components--collapsible-resizable-panels-only-panel-a-collapsible&viewMode=story");
  const panelA = page.getByRole("button", { name: "Panel A", exact: true });
  await expect(panelA).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("button", { name: "Panel B", exact: true })).toHaveCount(0);
  const content = page.getByText("Panel A의 title 영역을 눌러 이 패널만 접고 펼칠 수 있습니다.");
  await expect(content).toBeVisible();
  await panelA.click(); await expect(panelA).toHaveAttribute("aria-expanded", "false");
  await expect(content).toHaveCount(0);
  await expect(page.getByText("Panel B content")).toBeVisible();
  await panelA.click(); await expect(content).toBeVisible();
});


test("collapsed chat preserves its own input undo history", async ({ page }) => {
  await openDemo(page, "persistent-panels-pixels");
  await page.getByRole("button", { name: "Expand chat", exact: true }).click();
  const chat = page.getByRole("textbox", { name: "Chat draft", exact: true });
  await chat.fill("chat draft"); await chat.press("End"); await chat.pressSequentially("!");
  await page.getByRole("button", { name: "Toggle chat", exact: true }).click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect(chat).toHaveValue("chat draft!");
  await chat.focus(); await chat.press("ControlOrMeta+z"); await expect(chat).toHaveValue("chat draft");
});
