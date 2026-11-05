import { MOCK_SCAN_MS } from "@core/mock/disk-space";
import { MOCK_HOST_ID } from "@core/mock/fixtures";
import { unwrap } from "@core/transport";
import { screen, waitFor, within } from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import DiskRoute from "./route";

describe("Disk route report tab parameter", () => {
  it.each(["largest", "bogus", ""])(
    "opens tab=%s on the finished report",
    async (tab) => {
      const r = renderWithProviders(<DiskRoute />, {
        route: `/dashboard/disk?view=space&vol=/&tab=${tab}`,
        transportOptions: { spaceScan: "/" },
      });
      expect(
        await screen.findByRole("button", { name: "Scan again" })
      ).toBeVisible();
      expect(
        await screen.findByRole("img", { name: /^Treemap of Macintosh HD/ })
      ).toBeVisible();
      expect(
        await screen.findByRole("table", {
          name: tab === "largest" ? "Largest files" : "Folder contents",
        })
      ).toBeVisible();
      expect(
        screen.getByRole("radio", {
          name: tab === "largest" ? "Largest files" : "Contents",
        })
      ).toHaveAttribute("aria-checked", "true");
      expect(screen.queryByText(/Scanning · about/)).toBeNull();
      if (tab === "largest") {
        await r.user.click(
          within(
            screen.getByRole("img", { name: /^Treemap of Macintosh HD/ })
          ).getByText("Users › riley", { exact: true })
        );
        expect(
          await screen.findByRole("table", { name: "Folder contents" })
        ).toBeVisible();
        expect(screen.getByRole("radio", { name: "Contents" })).toHaveAttribute(
          "aria-checked",
          "true"
        );
      }
    }
  );

  it("rescans a seeded report with a fresh id", async () => {
    let now = Date.now();
    const r = renderWithProviders(<DiskRoute />, {
      route: "/dashboard/disk?view=space&vol=/&tab=largest",
      transportOptions: { spaceScan: "/", now: () => now },
    });
    await screen.findByRole("table", { name: "Largest files" });
    const previous = await unwrap(r.transport.diskSpaceReport(MOCK_HOST_ID));
    await r.user.click(screen.getByRole("button", { name: "Scan again" }));
    await screen.findByText(/Scanning · about/);
    expect(screen.getByRole("radio", { name: "Contents" })).toHaveAttribute(
      "aria-checked",
      "true"
    );
    expect(
      await screen.findByRole("table", { name: "Folder contents" })
    ).toBeVisible();
    const status = await unwrap(r.transport.diskSpaceStatus(MOCK_HOST_ID));
    expect(status.report_id).not.toBe(previous?.id);
    now += MOCK_SCAN_MS;
    await screen.findByRole(
      "button",
      { name: "Scan again" },
      { timeout: 3000 }
    );
    await waitFor(async () =>
      expect(
        await unwrap(r.transport.diskSpaceReport(MOCK_HOST_ID))
      ).toMatchObject({ id: status.report_id, volume: { mount: "/" } })
    );
  });
});
