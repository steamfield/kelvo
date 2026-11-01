import { expect, test } from "@playwright/test";

for (const theme of ["dark", "light"]) {
  test(`incompatible host release explains coordinated upgrade (${theme})`, async ({
    page,
  }, info) => {
    await page.setViewportSize({ width: 1280, height: 860 });
    await page.goto(
      `/?route=/dashboard/hosts&scenario=multi-host,remote-update-blocked&theme=${theme}&ticks=0`
    );
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const guide = page.getByRole("button", { name: "Upgrade guide" });
    await guide.scrollIntoViewIfNeeded();
    await expect(page.getByText(/Update agent cannot apply it/)).toBeVisible();
    await expect(guide).toBeEnabled();
    await guide.click();
    await expect(page.getByRole("alert")).toHaveCount(0);
    await page.screenshot({
      path: `test-results/screens/remote-update-blocked-${theme}-${info.project.name}.png`,
    });
  });
}

for (const theme of ["dark", "light"]) {
  test(`hosts behind this Mac raise a notice that links to Hosts (${theme})`, async ({
    page,
  }, info) => {
    await page.setViewportSize({ width: 1280, height: 860 });
    await page.goto(
      `/?window=dashboard&route=/dashboard/overview&scenario=hosts-outdated&theme=${theme}&ticks=0`
    );
    const notice = page
      .getByRole("region", { name: "Host updates" })
      .filter({ hasText: "4 hosts are running an older Kelvo." });
    await expect(notice).toBeVisible();
    await expect(
      page.getByRole("link", { name: /Hosts.*4 hosts need an update/ })
    ).toBeVisible();
    await page.screenshot({
      path: `test-results/screens/host-update-notice-${theme}-${info.project.name}.png`,
    });
    await notice.getByRole("button", { name: "Review hosts" }).click();
    await expect(page.getByRole("heading", { name: "Hosts" })).toBeVisible();
    await expect(notice).toHaveCount(0);
    await expect(
      page.getByText("Kelvo 0.0.9 (last reported) → 0.1.0 available")
    ).toBeVisible();
    await page.screenshot({
      path: `test-results/screens/hosts-versions-${theme}-${info.project.name}.png`,
    });

    await page.getByRole("button", { name: "Update all (2)" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("Offline: not updated")).toBeVisible();
    await dialog.getByRole("button", { name: "Update agents" }).click();
    await expect(dialog.getByText(/Requested on 2 of 2/)).toBeVisible();
    await dialog.getByRole("button", { name: "Done" }).click();

    const nav = page.getByRole("navigation", { name: "Dashboard sections" });
    await nav.getByRole("link", { name: /^Overview/ }).click();
    await page
      .getByRole("region", { name: "Host updates" })
      .filter({ hasText: "older Kelvo" })
      .getByRole("button", { name: "Dismiss" })
      .click();
    await nav.getByRole("link", { name: /^CPU/ }).click();
    await expect(page.getByRole("heading", { name: "CPU" })).toBeVisible();
    await expect(page.getByText("older Kelvo")).toHaveCount(0);
  });
}

test("a host that can't connect keeps the notice and asks for the update", async ({
  page,
}) => {
  await page.goto(
    "/?window=dashboard&route=/dashboard/overview&scenario=host-incompatible&ticks=0"
  );
  await expect(
    page.getByText("1 host can’t connect until it’s updated.")
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Dismiss" })).toHaveCount(0);
});

test("this Mac's own update comes before the hosts'", async ({ page }) => {
  await page.goto(
    "/?window=dashboard&route=/dashboard/overview&scenario=hosts-outdated&update=available&ticks=0"
  );
  const notice = page
    .getByRole("region", { name: "Host updates" })
    .filter({ hasText: "Update this Mac first" });
  await notice.getByRole("button", { name: "Open Settings" }).click();
  await expect(
    page.getByRole("button", { name: "Install and restart" })
  ).toBeVisible();
  await expect(notice).toHaveCount(0);
});
