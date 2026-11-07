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
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@tests/test-utils";
import { Profiler, useState } from "react";
import { useProcessRows } from "~/hooks/use-process-interest";
import { type ProcessSort, ProcessTable, processKey } from "./process-table";

function proc(
  over: Partial<LiveProcess> & Pick<LiveProcess, "pid" | "name">
): LiveProcess {
  return {
    start_time_us: 1_700_000_000_000_000 + over.pid,
    cpu_pct: 0,
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
    ...over,
  };
}

const ROWS: LiveProcess[] = [
  proc({ pid: 391, name: "WindowServer", cpu_pct: 33.6, threads: 22 }),
  proc({ pid: 2214, name: "Xcode", cpu_pct: 86.8, threads: 74 }),
  proc({ pid: 0, name: "kernel_task", cpu_pct: 43.4, threads: 612 }),
  proc({ pid: 5531, name: "node", cpu_pct: null, threads: 12 }),
];

function Harness({
  initial,
  rows = ROWS,
}: {
  initial: ProcessSort;
  rows?: readonly LiveProcess[];
}) {
  const [sort, setSort] = useState(initial);
  return (
    <ProcessTable
      rows={rows}
      columns={["name", "pid", "cpu", "threads"]}
      sort={sort}
      onSort={setSort}
      height={400}
      ariaLabel="Top processes"
    />
  );
}

function names(): string[] {
  const table = screen.getByRole("table", { name: "Top processes" });
  return within(table)
    .getAllByRole("row")
    .slice(1)
    .map((r) => within(r).getAllByRole("cell")[0]?.textContent ?? "")
    .filter(Boolean)
    .map((t) => t.slice(1)); // drop the initial chip
}

describe("ProcessTable", () => {
  it("sorts by % CPU descending with missing values last", () => {
    render(<Harness initial={{ by: "cpu", dir: "desc" }} />);
    expect(names()).toEqual(["Xcode", "kernel_task", "WindowServer", "node"]);
    expect(screen.getByRole("columnheader", { name: /% CPU/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
  });

  it("flips direction on the sorted column and starts text columns ascending", async () => {
    const user = userEvent.setup();
    render(<Harness initial={{ by: "cpu", dir: "desc" }} />);

    await user.click(screen.getByRole("button", { name: /% CPU/ }));
    expect(names()).toEqual(["WindowServer", "kernel_task", "Xcode", "node"]);

    await user.click(screen.getByRole("button", { name: /Process/ }));
    expect(names()).toEqual(["kernel_task", "node", "WindowServer", "Xcode"]);
    expect(
      screen.getByRole("columnheader", { name: /Process/ })
    ).toHaveAttribute("aria-sort", "ascending");
  });

  it("keys rows by pid and start time, so a reused pid is a new row", () => {
    const a = proc({ pid: 42, name: "a", start_time_us: 1 });
    const b = proc({ pid: 42, name: "b", start_time_us: 2 });
    expect(processKey(a)).not.toBe(processKey(b));

    const { container, rerender } = render(
      <ProcessTable
        rows={[a]}
        columns={["name"]}
        sort={{ by: "name", dir: "asc" }}
        onSort={() => {}}
        height={200}
        ariaLabel="p"
      />
    );
    const before = container.querySelector("tr[data-key]");
    expect(before).toHaveAttribute("data-key", "42:1");
    rerender(
      <ProcessTable
        rows={[b]}
        columns={["name"]}
        sort={{ by: "name", dir: "asc" }}
        onSort={() => {}}
        height={200}
        ariaLabel="p"
      />
    );
    expect(container.querySelector("tr[data-key]")).toHaveAttribute(
      "data-key",
      "42:2"
    );
  });

  it("sorts through the header without mutating its input", () => {
    const rows = Object.freeze([...ROWS]);
    const copy = [...rows];
    render(<Harness initial={{ by: "cpu", dir: "desc" }} rows={rows} />);
    fireEvent.click(
      within(screen.getByRole("columnheader", { name: "Threads" })).getByRole(
        "button"
      )
    );
    expect(names()).toEqual(["kernel_task", "Xcode", "WindowServer", "node"]);
    fireEvent.click(
      within(screen.getByRole("columnheader", { name: /Threads/ })).getByRole(
        "button"
      )
    );
    expect(
      screen.getByRole("columnheader", { name: /Threads/ })
    ).toHaveAttribute("aria-sort", "ascending");
    expect(names()).toEqual(["node", "WindowServer", "Xcode", "kernel_task"]);
    expect(rows).toEqual(copy);
  });

  it("breaks ties and gap pairs by numeric PID across reordered frames", () => {
    const rows = [
      proc({ pid: 10, name: "ten", cpu_pct: 2 }),
      proc({ pid: 2, name: "two", cpu_pct: 2 }),
      proc({ pid: 30, name: "thirty", cpu_pct: null }),
      proc({ pid: 20, name: "twenty", cpu_pct: null }),
    ];
    const props = {
      columns: ["name", "cpu"] as const,
      onSort: () => {},
      height: 300,
      ariaLabel: "Top processes",
    };
    const { rerender } = render(
      <ProcessTable {...props} rows={rows} sort={{ by: "cpu", dir: "desc" }} />
    );
    expect(names()).toEqual(["two", "ten", "twenty", "thirty"]);
    rerender(
      <ProcessTable
        {...props}
        rows={[...rows].reverse()}
        sort={{ by: "cpu", dir: "asc" }}
      />
    );
    expect(names()).toEqual(["two", "ten", "twenty", "thirty"]);
  });

  it("preserves case-sensitive User order and prop-driven column order", () => {
    const rows = [
      proc({ pid: 9, name: "a", user: "alice" }),
      proc({ pid: 2, name: "b", user: "Zed" }),
    ];
    function Users({
      columns,
    }: {
      columns: readonly ("name" | "user" | "pid")[];
    }) {
      const [sort, setSort] = useState<ProcessSort>({ by: "name", dir: "asc" });
      return (
        <ProcessTable
          rows={rows}
          sort={sort}
          onSort={setSort}
          height={200}
          ariaLabel="Users"
          columns={columns}
        />
      );
    }
    const { rerender } = render(<Users columns={["name", "user", "pid"]} />);
    fireEvent.click(
      within(screen.getByRole("columnheader", { name: "User" })).getByRole(
        "button"
      )
    );
    expect(screen.getByRole("columnheader", { name: /User/ })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );
    expect(
      screen
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("cell")[1]?.textContent)
    ).toEqual(["Zed", "alice"]);
    rerender(<Users columns={["pid", "user", "name"]} />);
    expect(
      screen.getAllByRole("columnheader").map((head) => head.textContent)
    ).toEqual(["PID", "User ↑", "Process"]);
    expect(
      screen
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("cell")[0]?.textContent)
    ).toEqual(["2", "9"]);
    fireEvent.click(
      within(screen.getByRole("columnheader", { name: /User/ })).getByRole(
        "button"
      )
    );
    expect(screen.getByRole("columnheader", { name: /User/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    expect(
      screen
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("cell")[1]?.textContent)
    ).toEqual(["alice", "Zed"]);
  });

  it("commits once per process frame with controlled sorting", async () => {
    const transport = createMockTransport({ autoTick: false });
    let commits = 0;
    function StreamingTable() {
      const rows = useProcessRows() ?? ROWS;
      const [sort, setSort] = useState<ProcessSort>({ by: "cpu", dir: "desc" });
      return (
        <Profiler id="process-table" onRender={() => commits++}>
          <ProcessTable
            rows={rows}
            columns={["name", "cpu"]}
            sort={sort}
            onSort={setSort}
            height={400}
            ariaLabel="Streaming processes"
          />
        </Profiler>
      );
    }
    const result = renderWithProviders(<StreamingTable />, { transport });
    await waitFor(() => expect(transport.subscriberCount()).toBe(1));
    act(() => transport.push({ kind: "processes", ts_ms: 1000, rows: ROWS }));
    await screen.findByRole("cell", { name: "86.8" });
    for (let tick = 1; tick <= 3; tick++) {
      const before = commits;
      act(() =>
        transport.push({
          kind: "processes",
          ts_ms: 1000 + tick * 1000,
          rows: ROWS.map((p) => ({ ...p, cpu_pct: tick + p.pid })),
        })
      );
      expect(
        screen.getByRole("cell", { name: `${2214 + tick}.0` })
      ).toBeVisible();
      expect(commits - before).toBe(1);
    }
    result.unmount();
    transport.dispose();
  });
});
