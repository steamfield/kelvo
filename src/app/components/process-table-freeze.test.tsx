import type { LiveProcess } from "@core/generated/bindings";
import { createMockTransport } from "@core/mock-transport";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { useProcessRows } from "~/hooks/use-process-interest";
import {
  diskTotal,
  frozenOrder,
  type ProcessSort,
  ProcessTable,
  processKey,
} from "./process-table";

function proc(pid: number, name: string, cpu: number): LiveProcess {
  return {
    pid,
    start_time_us: pid,
    name,
    cpu_pct: cpu,
    mem_bytes: 0,
    compressed_bytes: null,
    threads: 1,
    idle_wakeups_per_s: null,
    energy: null,
    disk_read_bps: null,
    disk_write_bps: null,
    net_rx_bps: null,
    net_tx_bps: null,
    gpu_pct: null,
    ports: null,
    user: "me",
    refusal: null,
  };
}

function table(rows: LiveProcess[]) {
  return (
    <ProcessTable
      rows={rows}
      columns={["name", "cpu"]}
      sort={{ by: "cpu", dir: "desc" }}
      onSort={() => {}}
      height={300}
      freezeOrderOnHover
      ariaLabel="Processes"
    />
  );
}

function names(): string[] {
  const t = screen.getByRole("table", { name: "Processes" });
  return within(t)
    .getAllByRole("row")
    .slice(1)
    .map((r) => within(r).queryAllByRole("cell")[0]?.textContent ?? "")
    .filter(Boolean)
    .map((s) => s.slice(1));
}

describe("frozenOrder", () => {
  it("keeps the frozen order, drops gone rows and appends new ones", () => {
    const a = proc(1, "a", 1);
    const b = proc(2, "b", 2);
    const c = proc(3, "c", 3);
    const out = frozenOrder([c, a], [processKey(a), processKey(b)]);
    expect(out.map((p) => p.name)).toEqual(["a", "c"]);
  });
});

describe("ProcessTable order freeze", () => {
  it("releases hover order when Selected only unmounts the live body", async () => {
    const transport = createMockTransport({ autoTick: false });
    function SelectableTable() {
      const rows = useProcessRows() ?? [];
      const [sort, setSort] = useState<ProcessSort>({ by: "cpu", dir: "desc" });
      return (
        <ProcessTable
          rows={rows}
          columns={["name", "cpu"]}
          sort={sort}
          onSort={setSort}
          height={300}
          selectable
          freezeOrderOnHover
          ariaLabel="Processes"
        />
      );
    }
    const result = renderWithProviders(<SelectableTable />, { transport });
    await waitFor(() => expect(transport.subscriberCount()).toBe(1));
    act(() =>
      transport.push({
        kind: "processes",
        ts_ms: 1000,
        rows: [proc(1, "a", 50), proc(2, "b", 10)],
      })
    );
    fireEvent.pointerEnter(screen.getByTestId("process-table-scroll"));
    fireEvent.click(screen.getByRole("checkbox", { name: "Select a (1)" }));
    // Keyboard activation does not move the pointer off the live table.
    fireEvent.click(screen.getByRole("button", { name: "Selected only" }));
    fireEvent.pointerLeave(
      screen.getByRole("table", { name: "Selected processes" })
    );
    fireEvent.click(screen.getByRole("button", { name: "Selected only" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    act(() =>
      transport.push({
        kind: "processes",
        ts_ms: 2000,
        rows: [proc(1, "a", 5), proc(2, "b", 90)],
      })
    );
    const table = screen.getByRole("table", { name: "Processes" });
    expect(
      within(table)
        .getAllByRole("row")
        .slice(1)
        .map((row) =>
          within(row).getAllByRole("cell")[1]?.textContent?.slice(1)
        )
    ).toEqual(["b", "a"]);
    result.unmount();
    transport.dispose();
  });

  it("holds row order while the pointer is over the table", () => {
    const { container, rerender } = render(
      table([proc(1, "a", 50), proc(2, "b", 10)])
    );
    expect(names()).toEqual(["a", "b"]);

    const scroller = container.firstElementChild as HTMLElement;
    fireEvent.pointerEnter(scroller);
    rerender(table([proc(1, "a", 5), proc(2, "b", 90), proc(3, "c", 99)]));
    // Values update in place; order holds; the new row goes last.
    expect(names()).toEqual(["a", "b", "c"]);
    expect(screen.getByRole("cell", { name: "5.0" })).toBeInTheDocument();

    fireEvent.pointerLeave(scroller);
    expect(names()).toEqual(["c", "b", "a"]);
  });

  it("re-sorts at once when a header is clicked while hovering", () => {
    let sort = { by: "cpu" as const, dir: "desc" as "asc" | "desc" };
    const rows = [proc(1, "a", 50), proc(2, "b", 10)];
    const view = () => (
      <ProcessTable
        rows={rows}
        columns={["name", "cpu"]}
        sort={sort}
        onSort={(s) => {
          sort = s as typeof sort;
        }}
        height={300}
        freezeOrderOnHover
        ariaLabel="Processes"
      />
    );
    const { container, rerender } = render(view());
    fireEvent.pointerEnter(container.firstElementChild as HTMLElement);
    fireEvent.click(screen.getByRole("button", { name: /% CPU/ }));
    rerender(view());
    expect(names()).toEqual(["b", "a"]);
  });
});

describe("diskTotal", () => {
  it("adds read and write and is null when either is", () => {
    const p = proc(1, "a", 0);
    expect(diskTotal(p)).toBeNull();
    expect(diskTotal({ ...p, disk_read_bps: 3 })).toBeNull();
    expect(diskTotal({ ...p, disk_read_bps: 3, disk_write_bps: 4 })).toBe(7);
  });
});
