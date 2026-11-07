import { seriesKey, sk } from "@core/series-key";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { useHeld } from "~/hooks/use-ring";
import { type HostStore, useHostStore } from "~/stores/host-store";
import { type InterfaceRow, InterfaceTable } from "./interface-table";

const rows: InterfaceRow[] = [
  { id: "en9", kind: "Wi-Fi", rxBps: 20, txBps: 1, rxBytes: 30, txBytes: 1 },
  {
    id: "en10",
    kind: "Ethernet",
    rxBps: 10,
    txBps: 2,
    rxBytes: 40,
    txBytes: 2,
  },
  {
    id: "gap",
    kind: "VPN",
    rxBps: null,
    txBps: null,
    rxBytes: null,
    txBytes: null,
    disconnected: true,
  },
];
const names = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map((r) => within(r).getAllByRole("cell")[0]?.textContent);

it.each([
  ["Interface", ["en10", "en9", "gap"], ["gap", "en9", "en10"], "ascending"],
  ["Kind", ["en10", "gap", "en9"], ["en9", "gap", "en10"], "ascending"],
  ["Down", ["en9", "en10", "gap"], ["en10", "en9", "gap"], "descending"],
  ["Up", ["en10", "en9", "gap"], ["en9", "en10", "gap"], "descending"],
  ["Received", ["en10", "en9", "gap"], ["en9", "en10", "gap"], "descending"],
  ["Sent", ["en10", "en9", "gap"], ["en9", "en10", "gap"], "descending"],
])(
  "sorts %s both ways with gaps last and source order before sorting",
  (label, first, second, direction) => {
    render(<InterfaceTable rows={rows} units="MBps" />);
    expect(names()).toEqual(["en9", "en10", "gap"]);
    expect(screen.getByText("Disconnected")).toBeVisible();
    const header = () =>
      screen.getByRole("columnheader", { name: new RegExp(label) });
    fireEvent.click(within(header()).getByRole("button"));
    expect(header()).toHaveAttribute("aria-sort", direction);
    expect(names()).toEqual(first);
    fireEvent.click(within(header()).getByRole("button"));
    expect(header()).toHaveAttribute(
      "aria-sort",
      direction === "ascending" ? "descending" : "ascending"
    );
    expect(names()).toEqual(second);
  }
);

it("holds hover order across pushed frames, updates values and applies a header change immediately", async () => {
  const keys = [
    sk("net.rx", { iface: "en9" }),
    sk("net.rx", { iface: "en10" }),
  ];
  let store: HostStore | undefined;
  function LiveInterfaces() {
    store = useHostStore();
    const held = useHeld(keys);
    return (
      <InterfaceTable
        rows={rows.map((r) => ({
          ...r,
          rxBps: held[sk("net.rx", { iface: r.id })] ?? null,
        }))}
        units="MBps"
      />
    );
  }
  const result = renderWithProviders(<LiveInterfaces />);
  await waitFor(() => expect(result.transport.subscriberCount()).toBe(1));
  act(() =>
    result.transport.push({
      kind: "layout",
      layout_no: 99,
      kinds: ["rate", "rate"],
      series: [
        seriesKey("net.rx", { iface: "en9" }),
        seriesKey("net.rx", { iface: "en10" }),
      ],
    })
  );
  let ts = store?.getState().lastTsMs ?? 0;
  const frame = (values: number[]) =>
    act(() =>
      result.transport.push({
        kind: "frame",
        layout_no: 99,
        timeline: store?.getState().timeline ?? 0,
        ts_ms: ++ts,
        values,
        held: values,
      })
    );
  frame([20e6, 10e6]);
  fireEvent.click(screen.getByRole("button", { name: "Down" }));
  expect(names()).toEqual(["en9", "en10", "gap"]);
  fireEvent.pointerEnter(screen.getByRole("table"));
  frame([5e6, 90e6]);
  expect(names()).toEqual(["en9", "en10", "gap"]);
  expect(screen.getByRole("cell", { name: "90.0 MB/s" })).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: /Down/ }));
  expect(names()).toEqual(["en9", "en10", "gap"]);
  expect(screen.getByRole("columnheader", { name: /Down/ })).toHaveAttribute(
    "aria-sort",
    "ascending"
  );
  fireEvent.click(screen.getByRole("button", { name: /Down/ }));
  expect(names()).toEqual(["en10", "en9", "gap"]);
  frame([100e6, 1e6]);
  expect(names()).toEqual(["en10", "en9", "gap"]);
  fireEvent.pointerLeave(screen.getByRole("table"));
  expect(names()).toEqual(["en9", "en10", "gap"]);
});
