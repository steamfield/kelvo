import { createAppTransport } from "./app-transport";
import { MOCK_SCAN_MS } from "./mock/disk-space";
import { MOCK_HOST_ID } from "./mock/fixtures";
import type { CommandResult, Transport } from "./transport";

function data<T>(result: CommandResult<T>): T {
  if (result.status === "error") throw new Error(result.error.kind);
  return result.data;
}

describe("mock transport query parameters", () => {
  let transport: Transport;
  afterEach(() => {
    if (
      transport &&
      "dispose" in transport &&
      typeof transport.dispose === "function"
    )
      transport.dispose();
  });

  it("opens a finished boot report immediately and can rescan normally", async () => {
    transport = await createAppTransport("?space_scan=/&ticks=0");
    const status = data(await transport.diskSpaceStatus(MOCK_HOST_ID));
    expect(status).toMatchObject({
      phase: "done",
      volume: "/",
      elapsed_ms: MOCK_SCAN_MS,
    });
    const report = data(await transport.diskSpaceReport(MOCK_HOST_ID));
    expect(report).toMatchObject({
      id: status.report_id,
      volume: { mount: "/" },
      duration_ms: MOCK_SCAN_MS,
    });
    expect(report?.finished_ms).toEqual(expect.any(Number));
    if (!report) throw new Error("missing report");
    const tree = data(
      await transport.diskSpaceSubtree(MOCK_HOST_ID, report.id, null)
    );
    expect(tree.folder.done).toBe(true);
    expect(tree.children.length).toBeGreaterThan(0);
    expect(
      data(await transport.diskSpaceLargestFiles(MOCK_HOST_ID, report.id))
        .length
    ).toBeGreaterThan(0);
    const rescan = data(await transport.diskSpaceStart(MOCK_HOST_ID, "/", []));
    expect(rescan.phase).toBe("scanning");
    expect(rescan.report_id).not.toBe(report.id);
    expect(data(await transport.diskSpaceReport(MOCK_HOST_ID))?.id).toBe(
      report.id
    );
  });

  it.each(["/missing", "", "relative", "%ZZ"])(
    "ignores an unknown scan mount %s",
    async (mount) => {
      transport = await createAppTransport(
        `?ticks=0&space_scan=${encodeURIComponent(mount)}`
      );
      expect(data(await transport.diskSpaceStatus(MOCK_HOST_ID)).phase).toBe(
        "idle"
      );
      expect(data(await transport.diskSpaceReport(MOCK_HOST_ID))).toBeNull();
    }
  );

  it("seeds only volumes offered by each host", async () => {
    transport = await createAppTransport(
      "?ticks=0&scenario=linux-host&space_scan=/boot"
    );
    expect(data(await transport.diskSpaceStatus(MOCK_HOST_ID)).phase).toBe(
      "idle"
    );
    expect(data(await transport.diskSpaceStatus("remote-linux"))).toMatchObject(
      { phase: "done", volume: "/boot" }
    );
  });

  it.each([
    ["24h", { relative: 86_400_000 }],
    ["7d", { relative: 604_800_000 }],
    ["30d", { relative: 2_592_000_000 }],
    ["5m", "5m"],
    ["15m", "15m"],
    ["30m", "30m"],
    ["1h", "1h"],
    ["bogus", "15m"],
    ["", "15m"],
  ])("initializes chart_window=%s", async (value, expected) => {
    transport = await createAppTransport(`?ticks=0&chart_window=${value}`);
    expect(
      (await transport.getSettings()).settings.general.chart_window
    ).toEqual(expected);
  });
});
