import { expect, test } from "@playwright/test";

for (const theme of ["light", "dark"]) {
  test(`a never-connected host keeps navigation and recovery actions (${theme})`, async ({
    page,
  }, info) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 833 });
    await page.goto(
      `/?route=/dashboard/hosts&scenario=host-unreachable&theme=${theme}&ticks=0`
    );
    const row = page
      .getByRole("heading", { name: "unreachable", exact: true })
      .locator("..")
      .locator("..");
    await row
      .getByRole("button", { name: "View metrics", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "Overview", exact: true })
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Can’t connect to unreachable" })
    ).toBeVisible();
    await expect(page.getByRole("navigation")).toBeVisible();
    await expect(page.getByText("Connecting", { exact: true })).toBeVisible();
    await expect(
      page.getByText("Sampling every 0s", { exact: true })
    ).toHaveCount(0);
    await page.screenshot({
      path: `test-results/screens/remote-unreachable-${theme}-${info.project.name}.png`,
    });
    await page.getByText("Technical details", { exact: true }).click();
    await expect(page.getByText(/SSH command failed \(127\)/)).toBeVisible();
    await page
      .getByRole("button", { name: "Retry connection", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Retry connection", exact: true })
    ).toBeEnabled();
    await page.getByRole("link", { name: "CPU", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "CPU", exact: true })
    ).toBeVisible();
    await expect(
      page.getByText("This view hit a snag", { exact: true })
    ).toHaveCount(0);
    await page.screenshot({
      path: `test-results/screens/remote-empty-cpu-${theme}-${info.project.name}.png`,
    });
    await page.getByRole("link", { name: "Open Hosts", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "Hosts", exact: true })
    ).toBeVisible();
    expect(errors).toEqual([]);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth
      )
    ).toBe(true);
  });
}
