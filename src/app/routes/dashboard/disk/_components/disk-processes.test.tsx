import type { LiveProcess, ProcessUsage } from "@core/generated/bindings";
import { createMockTransport } from "@core/mock-transport";
import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { BrushProvider } from "~/stores/brush-store";
import { DiskProcesses } from "./disk-processes";

const NOW = 1_700_000_005_000;
const RANGE = { fromMs: NOW - 15_000, toMs: NOW - 5000 };

it.each(["selected", "live"])(
  "process links preserve the current %s brush before its URL mirror catches up",
  async (mode) => {
    const selected = mode === "selected";
    renderWithProviders(
      <BrushProvider initial={selected ? RANGE : null}>
        <DiskProcesses windowMs={300_000} />
      </BrushProvider>,
      {
        transportOptions: { now: () => NOW, autoTick: false },
        route: `/dashboard/disk?device=disk7&response=ops${selected ? "" : "&from=1000&to=2000"}`,
      }
    );
    const link = await screen.findByRole("link", {
      name: selected ? "Inspect clang, PID 6120" : "Inspect Xcode, PID 2214",
    });
    const destination = new URL(
      link.getAttribute("href") ?? "",
      "http://localhost"
    );
    const back = destination.searchParams.get("back");
    expect(back).not.toBeNull();
    const returnTo = new URL(back ?? "", "http://localhost");
    expect(returnTo.pathname).toBe("/dashboard/disk");
    expect(returnTo.searchParams.get("device")).toBe("disk7");
    expect(returnTo.searchParams.get("response")).toBe("ops");
    for (const url of [destination, returnTo]) {
      expect(url.searchParams.get("from")).toBe(
        selected ? String(RANGE.fromMs) : null
      );
      expect(url.searchParams.get("to")).toBe(
        selected ? String(RANGE.toMs) : null
      );
    }
  }
);

const proc = (
  pid: number,
  read: number | null,
  write: number | null,
  name = `process${pid}`
): LiveProcess => ({
  pid,
  start_time_us: pid,
  name,
  cpu_pct: 0,
  mem_bytes: 0,
  compressed_bytes: null,
  threads: 1,
  idle_wakeups_per_s: null,
  energy: null,
  disk_read_bps: read,
  disk_write_bps: write,
  net_rx_bps: null,
  net_tx_bps: null,
  gpu_pct: null,
  ports: null,
  user: "me",
  refusal: null,
});
const processNames = () =>
  within(screen.getByRole("table", { name: /^Disk processes/ }))
    .queryAllByRole("link")
    .map((link) => link.textContent);
const list = () =>
  renderWithProviders(
    <BrushProvider>
      <DiskProcesses windowMs={300_000} />
    </BrushProvider>,
    { transportOptions: { now: () => NOW, autoTick: false } }
  );

it("sorts the complete process set before slicing the top ten and searching", async () => {
  const r = list();
  await screen.findByRole("table", { name: "Disk processes · Live" });
  act(() =>
    r.transport.push({
      kind: "processes",
      ts_ms: NOW,
      rows: [
        ...Array.from({ length: 10 }, (_, i) => proc(i + 1, 1, 1000 + i)),
        proc(11, 500, 0),
      ],
    })
  );
  expect(processNames()).not.toContain("process11");
  fireEvent.click(screen.getByRole("button", { name: "Read/s" }));
  expect(screen.getByRole("columnheader", { name: /Read\/s/ })).toHaveAttribute(
    "aria-sort",
    "descending"
  );
  expect(processNames()[0]).toBe("process11");
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "process1" },
  });
  expect(processNames()).toEqual(["process11", "process1", "process10"]);
  fireEvent.click(screen.getByRole("button", { name: /Read\/s/ }));
  expect(processNames()).toEqual(["process1", "process10", "process11"]);
});

it.each(["pointer", "focus"])(
  "holds live order through %s, updates values and applies sorting immediately",
  async (gesture) => {
    const r = list();
    await screen.findByRole("table", { name: "Disk processes · Live" });
    const push = (a: number, b: number) =>
      act(() =>
        r.transport.push({
          kind: "processes",
          ts_ms: NOW,
          rows: [proc(1, a, 0, "Alpha"), proc(2, b, 0, "Beta")],
        })
      );
    push(20e6, 10e6);
    const scroller = () =>
      screen.getByRole("table", { name: /^Disk processes/ }).parentElement;
    const surface = scroller();
    if (!surface) throw new Error("Missing process viewport");
    if (gesture === "pointer") fireEvent.pointerEnter(surface);
    else
      act(() =>
        screen.getByRole("link", { name: "Inspect Alpha, PID 1" }).focus()
      );
    push(5e6, 90e6);
    expect(processNames()).toEqual(["Alpha", "Beta"]);
    expect(
      screen.getAllByRole("cell", { name: "90.0 MB/s" }).length
    ).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Read/s" }));
    expect(processNames()).toEqual(["Beta", "Alpha"]);
    push(100e6, 1e6);
    expect(processNames()).toEqual(["Beta", "Alpha"]);
    if (gesture === "pointer") fireEvent.pointerLeave(surface);
    else
      act(() =>
        screen.getByRole("link", { name: "Inspect Alpha, PID 1" }).blur()
      );
    await waitFor(() => expect(processNames()).toEqual(["Alpha", "Beta"]));
  }
);

it.each(["Read/s", "Write/s", "Total/s", "Process"])(
  "sorts live %s both ways with pid ties and gaps last",
  async (label) => {
    const r = list();
    await screen.findByRole("table", { name: "Disk processes · Live" });
    act(() =>
      r.transport.push({
        kind: "processes",
        ts_ms: NOW,
        rows: [
          proc(9, 20, 1, "proc9"),
          proc(10, 10, 2, "Proc10"),
          proc(11, null, null, "Waiting"),
        ],
      })
    );
    const first =
      label === "Process" || label === "Write/s"
        ? ["Proc10", "proc9", "Waiting"]
        : ["proc9", "Proc10", "Waiting"];
    // Total is already descending, so its first click flips to ascending.
    const total = label === "Total/s";
    const expected = total ? ["Proc10", "proc9", "Waiting"] : first;
    const header = () =>
      screen.getByRole("columnheader", { name: new RegExp(label) });
    fireEvent.click(within(header()).getByRole("button"));
    expect(processNames()).toEqual(expected);
    expect(header()).toHaveAttribute(
      "aria-sort",
      label === "Process" || total ? "ascending" : "descending"
    );
    fireEvent.click(within(header()).getByRole("button"));
    expect(processNames()).toEqual(
      label === "Process"
        ? ["Waiting", "proc9", "Proc10"]
        : [...expected.slice(0, 2).reverse(), "Waiting"]
    );
  }
);

it.each(["Read", "Written", "Total", "Process"])(
  "sorts range %s both ways, retaining gaps, links and average sub-lines",
  async (label) => {
    const transport = createMockTransport({ now: () => NOW, autoTick: false });
    const query = transport.queryUsageByApp.bind(transport);
    const usage = (
      pid: number,
      name: string,
      read: number | null,
      write: number | null
    ): ProcessUsage => ({
      pid,
      start_time_us: pid,
      name,
      cpu_avg_pct: null,
      gpu_avg_pct: null,
      mem_peak_bytes: 0,
      read_bytes: read,
      write_bytes: write,
      read_avg_bps: read,
      write_avg_bps: write,
      total_avg_bps:
        read === null || write === null ? null : (read + write) / 10,
      energy_j: null,
      avg_w: null,
      running: true,
      refusal: null,
    });
    transport.queryUsageByApp = async (...args) => {
      const result = await query(...args);
      if (result.status === "error") return result;
      const app = result.data.apps[0];
      if (!app) throw new Error("Missing usage fixture");
      return {
        ...result,
        data: {
          ...result.data,
          apps: [
            {
              ...app,
              processes: [
                usage(9, "proc9", 20e6, 1e6),
                usage(10, "Proc10", 10e6, 2e6),
                usage(11, "Waiting", null, null),
              ],
            },
          ],
        },
      };
    };
    renderWithProviders(
      <BrushProvider initial={RANGE}>
        <DiskProcesses windowMs={300_000} />
      </BrushProvider>,
      { transport }
    );
    await screen.findByRole("link", { name: "Inspect proc9, PID 9" });
    expect(processNames()).toEqual(["proc9", "Proc10", "Waiting"]);
    const header = () =>
      screen.getByRole("columnheader", { name: new RegExp(`^${label}`) });
    fireEvent.click(within(header()).getByRole("button"));
    const ascending = label === "Total" || label === "Process";
    const first =
      label === "Read"
        ? ["proc9", "Proc10", "Waiting"]
        : ["Proc10", "proc9", "Waiting"];
    expect(header()).toHaveAttribute(
      "aria-sort",
      ascending ? "ascending" : "descending"
    );
    expect(processNames()).toEqual(first);
    fireEvent.click(within(header()).getByRole("button"));
    expect(header()).toHaveAttribute(
      "aria-sort",
      ascending ? "descending" : "ascending"
    );
    expect(processNames()).toEqual(
      label === "Process"
        ? ["Waiting", "proc9", "Proc10"]
        : [...first.slice(0, 2).reverse(), "Waiting"]
    );
    expect(
      screen.getByRole("columnheader", { name: /^Total/ })
    ).toHaveTextContent("Avg/s");
    expect(
      screen.getByRole("link", { name: "Inspect proc9, PID 9" })
    ).toHaveAttribute("href", expect.stringContaining(`from=${RANGE.fromMs}`));
  }
);

it("breaks equal names and rates by numeric pid in both directions", async () => {
  const r = list();
  await screen.findByRole("table", { name: "Disk processes · Live" });
  act(() =>
    r.transport.push({
      kind: "processes",
      ts_ms: NOW,
      rows: [proc(10, 1, 1, "same"), proc(2, 1, 1, "Same")],
    })
  );
  const pids = () =>
    within(screen.getByRole("table", { name: /^Disk processes/ }))
      .getAllByRole("link")
      .map((link) => link.getAttribute("aria-label"));
  expect(pids()).toEqual(["Inspect Same, PID 2", "Inspect same, PID 10"]);
  for (const label of ["Process", "Read/s", "Write/s", "Total/s"]) {
    for (let click = 0; click < 2; click++) {
      fireEvent.click(
        screen.getByRole("button", { name: new RegExp(`^${label}`) })
      );
      expect(pids()).toEqual(["Inspect Same, PID 2", "Inspect same, PID 10"]);
    }
  }
});
