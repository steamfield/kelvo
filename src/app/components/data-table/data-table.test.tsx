import type { SortingState } from "@tanstack/react-table";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";
import { columnHelper, DataTable, DataTableHead, gap, useDataTable } from ".";

interface Item {
  id: string;
  name: string;
  value: number | null;
}
const helper = columnHelper<Item>();
const columns = helper.columns([
  helper.accessor("name", {
    header: "Name",
    sortFn: "text",
    meta: { align: "left" },
  }),
  helper.accessor((row) => gap(row.value), {
    id: "value",
    header: "Value",
    meta: { align: "right" },
  }),
  helper.display({ id: "action", header: "Action", meta: { align: "left" } }),
]);
const getRowId = (row: Item) => row.id;
const a = { id: "a", name: "proc9", value: 2 };
const b = { id: "b", name: "Proc10", value: 1 };
const c = { id: "c", name: "gap", value: null };

function Harness({
  data = [a, b, c],
  initialSorting = [],
  manualSorting = false,
  paused = false,
}: {
  data?: Item[];
  initialSorting?: SortingState;
  manualSorting?: boolean;
  paused?: boolean;
}) {
  const [sorting, setSorting] = useState(initialSorting);
  const { table, rows } = useDataTable({
    data,
    columns,
    getRowId,
    sorting,
    onSortingChange: setSorting,
    manualSorting,
    hold: { paused, resetKey: JSON.stringify(sorting) },
  });
  return (
    <Table aria-label="Items">
      <TableHeader>
        <DataTableHead table={table} />
      </TableHeader>
      <DataTable
        table={table}
        rows={rows}
        after={
          <TableRow>
            <TableCell colSpan={3}>Remainder</TableCell>
          </TableRow>
        }
      />
    </Table>
  );
}

function names() {
  return within(screen.getByRole("table"))
    .getAllByRole("row")
    .slice(1)
    .map((row) => within(row).getAllByRole("cell")[0]?.textContent);
}

describe("DataTable", () => {
  it("orders distinct IDs that collate equally consistently across frames", () => {
    const composed = { id: "tie:é", name: "composed tie", value: 2 };
    const decomposed = { id: "tie:e\u0301", name: "decomposed tie", value: 2 };
    const composedGap = { id: "gap:é", name: "composed gap", value: null };
    const decomposedGap = {
      id: "gap:e\u0301",
      name: "decomposed gap",
      value: null,
    };
    const data = [composed, decomposed, composedGap, decomposedGap];
    const { rerender } = render(
      <Harness data={data} initialSorting={[{ id: "value", desc: true }]} />
    );
    const expected = [
      "decomposed tie",
      "composed tie",
      "decomposed gap",
      "composed gap",
      "Remainder",
    ];
    expect(names()).toEqual(expected);
    rerender(<Harness data={[...data].reverse()} />);
    expect(names()).toEqual(expected);
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(names()).toEqual(expected);
  });

  it("resets to the supplied uncontrolled initial sort", () => {
    const data = [b, c, a];
    function Resettable() {
      const { table, rows } = useDataTable({
        data,
        columns,
        getRowId,
        initialSorting: [{ id: "value", desc: true }],
      });
      return (
        <>
          <button type="button" onClick={() => table.resetSorting()}>
            Reset
          </button>
          <Table>
            <TableHeader>
              <DataTableHead table={table} />
            </TableHeader>
            <DataTable table={table} rows={rows} />
          </Table>
        </>
      );
    }
    render(<Resettable />);
    fireEvent.click(screen.getByRole("button", { name: "Name" }));
    expect(names()).toEqual(["gap", "Proc10", "proc9"]);
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(names()).toEqual(["proc9", "Proc10", "gap"]);
    expect(screen.getByRole("columnheader", { name: /Value/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
  });

  it("supports uncontrolled sorting and per-column direction overrides", () => {
    const customColumns = helper.columns([
      helper.accessor("name", {
        header: "Name",
        sortFn: "text",
        sortDescFirst: true,
        meta: { align: "left" },
      }),
      helper.display({
        id: "rank",
        header: "Rank",
        meta: { align: "right", ranked: true },
      }),
    ]);
    const data = [a, b];
    function Uncontrolled() {
      const { table, rows } = useDataTable({
        data,
        columns: customColumns,
        getRowId,
      });
      return (
        <Table>
          <TableHeader>
            <DataTableHead table={table} />
          </TableHeader>
          <DataTable table={table} rows={rows} />
        </Table>
      );
    }
    render(<Uncontrolled />);
    expect(names()).toEqual(["proc9", "Proc10"]);
    fireEvent.click(screen.getByRole("button", { name: /Name/ }));
    expect(names()).toEqual(["proc9", "Proc10"]);
    expect(screen.getByRole("columnheader", { name: /Name/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    expect(
      screen.getByRole("columnheader", { name: /Rank/ })
    ).toHaveTextContent("Rank ↓");
    expect(
      within(screen.getByRole("columnheader", { name: /Rank/ })).queryByRole(
        "button"
      )
    ).toBeNull();
  });

  it("keeps gaps last in both directions and the after row last", () => {
    render(<Harness initialSorting={[{ id: "value", desc: true }]} />);
    expect(names()).toEqual(["proc9", "Proc10", "gap", "Remainder"]);
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(names()).toEqual(["Proc10", "proc9", "gap", "Remainder"]);
  });

  it("keeps ties and gap pairs deterministic across reordered frames and directions", () => {
    const d = { id: "d", name: "gap2", value: null };
    const tied = { ...b, value: 2 };
    const { rerender } = render(
      <Harness
        data={[d, tied, c, a]}
        initialSorting={[{ id: "value", desc: true }]}
      />
    );
    expect(names()).toEqual(["proc9", "Proc10", "gap", "gap2", "Remainder"]);
    rerender(<Harness data={[c, a, d, tied]} />);
    expect(names()).toEqual(["proc9", "Proc10", "gap", "gap2", "Remainder"]);
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(names()).toEqual(["proc9", "Proc10", "gap", "gap2", "Remainder"]);
  });

  it("starts right columns descending and left columns ascending, flips without removing sort", () => {
    render(<Harness />);
    expect(names()).toEqual(["proc9", "Proc10", "gap", "Remainder"]);
    fireEvent.click(screen.getByRole("button", { name: "Value" }));
    expect(screen.getByRole("columnheader", { name: /Value/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(screen.getByRole("columnheader", { name: /Value/ })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(screen.getByRole("columnheader", { name: /Value/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    fireEvent.click(screen.getByRole("button", { name: "Name" }));
    expect(screen.getByRole("columnheader", { name: /Name/ })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );
    expect(
      screen.getByRole("columnheader", { name: /Value/ })
    ).not.toHaveAttribute("aria-sort");
    expect(names()).toEqual(["gap", "Proc10", "proc9", "Remainder"]);
    expect(
      within(screen.getByRole("columnheader", { name: "Action" })).queryByRole(
        "button"
      )
    ).toBeNull();
  });

  it("leaves manual sorting in source order", () => {
    render(
      <Harness
        data={[c, b, a]}
        manualSorting
        initialSorting={[{ id: "value", desc: true }]}
      />
    );
    expect(names()).toEqual(["gap", "Proc10", "proc9", "Remainder"]);
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(names()).toEqual(["gap", "Proc10", "proc9", "Remainder"]);
  });

  it("holds order while paused, appends new rows, drops departed rows and resets at once on sort", () => {
    const { rerender } = render(
      <Harness data={[a, b]} initialSorting={[{ id: "value", desc: true }]} />
    );
    rerender(<Harness data={[a, b]} paused />);
    const updated = { ...b, value: 9 };
    const newcomer = { id: "new", name: "new", value: 20 };
    rerender(<Harness data={[newcomer, a, updated]} paused />);
    expect(names()).toEqual(["proc9", "Proc10", "new", "Remainder"]);
    expect(screen.getByRole("cell", { name: "9" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Name" }));
    expect(names()).toEqual(["new", "Proc10", "proc9", "Remainder"]);
    rerender(<Harness data={[a, newcomer]} paused />);
    expect(names()).toEqual(["new", "proc9", "Remainder"]);
    fireEvent.click(screen.getByRole("button", { name: /Value/ }));
    expect(names()).toEqual(["new", "proc9", "Remainder"]);
    rerender(<Harness data={[{ ...a, value: 50 }, newcomer]} />);
    expect(names()).toEqual(["proc9", "new", "Remainder"]);
  });
});
