import { createAppTransport } from "@core/app-transport";
import { MOCK_HOST_ID } from "@core/mock/fixtures";
import type { Transport } from "@core/transport";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WindowControl } from "~/components/window-control";
import { useEvents } from "~/hooks/use-events";
import { useRangeNavigation } from "~/hooks/use-range-navigation";
import { QueryProvider } from "~/lib/query";
import { TransportProvider } from "~/lib/transport-context";
import { HostStoreProvider } from "~/stores/host-store";
import { SettingsProvider } from "~/stores/settings-store";
import { useTimelineData } from "./_hooks/use-timeline-data";
import { LANES } from "./_lib/lanes";

/** Timeline's navigation and picker, without the canvas plots Happy DOM lacks. */
function TimelineRange() {
  const navigation = useRangeNavigation();
  const data = useTimelineData(
    navigation.range ?? { windowMs: 86_400_000, endMs: null },
    LANES,
    1000
  );
  const events = useEvents(navigation.range?.windowMs ?? 86_400_000, null);
  const points = data.lanes[0]?.series["cpu.total"] ?? [];
  const quarter = (data.toMs - data.fromMs) / 4;
  return (
    <>
      <output aria-label="History across range">
        {points.some((p) => p.t < data.fromMs + quarter) &&
        points.some((p) => p.t > data.toMs - quarter)
          ? "Recorded data across range"
          : "Loading"}
      </output>
      <output aria-label="Events in range">
        {
          events.filter(
            (event) =>
              event.ts_ms >= data.fromMs &&
              event.ts_ms < data.toMs + data.bucketMs
          ).length
        }
      </output>
      <output aria-label="Timeline duration">
        {navigation.range?.windowMs}
      </output>
      <WindowControl onApply={(value) => void navigation.apply(value)} />
    </>
  );
}

describe("Timeline preview range", () => {
  let transport: Transport;
  afterEach(() => {
    cleanup();
    if (
      transport &&
      "dispose" in transport &&
      typeof transport.dispose === "function"
    )
      transport.dispose();
  });

  it.each([
    ["24h", "1d", "Last 1 day", 86_400_000],
    ["7d", "7d", "Last 7 days", 604_800_000],
    ["30d", "30d", "Last 30 days", 2_592_000_000],
  ])(
    "opens chart_window=%s with the matching range and selected preset",
    async (value, label, preset, duration) => {
      transport = await createAppTransport(`?ticks=0&chart_window=${value}`);
      render(
        <TransportProvider transport={transport}>
          <QueryProvider>
            <SettingsProvider>
              <HostStoreProvider hostId={MOCK_HOST_ID}>
                <TimelineRange />
              </HostStoreProvider>
            </SettingsProvider>
          </QueryProvider>
        </TransportProvider>
      );
      await waitFor(() =>
        expect(screen.getByLabelText("History across range")).toHaveTextContent(
          "Recorded data across range"
        )
      );
      await waitFor(() =>
        expect(screen.getByLabelText("Events in range")).toHaveTextContent("4")
      );
      const user = userEvent.setup();
      await user.click(
        await screen.findByRole("button", { name: `Time range: ${label}` })
      );
      expect(screen.getByLabelText("Timeline duration")).toHaveTextContent(
        String(duration)
      );
      expect(screen.getByRole("button", { name: preset })).toHaveAttribute(
        "aria-pressed",
        "true"
      );
    }
  );
});
