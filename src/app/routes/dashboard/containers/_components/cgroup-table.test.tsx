vi.mock("@tanstack/react-table", { spy: true });

import * as tanstack from "@tanstack/react-table";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import type { CgroupRow, CgroupSort } from "../_lib/cgroups";
import { CgroupTable } from "./cgroup-table";

const a: CgroupRow = {
  id: "docker/same",
  name: "same",
  kind: "docker",
  cpu: 10,
  mem: 100,
  memMax: null,
  ioRead: 5,
  ioWrite: 6,
  memPsi: 1,
};
const b: CgroupRow = { ...a, id: "service/same", kind: "service" };
const columns = [
  "name",
  "kind",
  "cpu",
  "mem",
  "ioRead",
  "ioWrite",
  "memPsi",
] as const;
const body = () => screen.getAllByRole("row").slice(1);

it("breaks equal names and metric gaps by id across frames and directions", () => {
  let update = () => {};
  function Harness() {
    const [rows, setRows] = useState([
      { ...b, cpu: null },
      { ...a, cpu: null },
    ]);
    const [sort, onSort] = useState<CgroupSort>({ by: "cpu", dir: "desc" });
    update = () => setRows([...rows].reverse());
    return (
      <CgroupTable
        rows={rows}
        columns={columns}
        sort={sort}
        onSort={onSort}
        other={null}
        bytes="GB"
        rate="MBps"
        empty="Empty"
      />
    );
  }
  render(<Harness />);
  const kinds = () =>
    body().map((row) => within(row).getAllByRole("cell")[1]?.textContent);
  expect(kinds()).toEqual(["Docker", "Service"]);
  act(() => update());
  expect(kinds()).toEqual(["Docker", "Service"]);
  fireEvent.click(
    within(screen.getByRole("columnheader", { name: /CPU/ })).getByRole(
      "button"
    )
  );
  expect(kinds()).toEqual(["Docker", "Service"]);
  fireEvent.click(
    within(screen.getByRole("columnheader", { name: "Name" })).getByRole(
      "button"
    )
  );
  expect(kinds()).toEqual(["Docker", "Service"]);
});

it("pins the remainder last, blanks kind and pressure, and updates its metrics", () => {
  const table = vi.mocked(tanstack.useTable);
  table.mockClear();
  let update = () => {};
  function Harness() {
    const [cpu, setCpu] = useState(90);
    const [sort, onSort] = useState<CgroupSort>({ by: "cpu", dir: "desc" });
    update = () => setCpu(80);
    return (
      <CgroupTable
        rows={[a, b]}
        columns={columns}
        sort={sort}
        onSort={onSort}
        other={{
          label: "25 others",
          row: { ...a, id: "other/_other", name: "_other", kind: "other", cpu },
        }}
        bytes="GB"
        rate="MBps"
        empty="Empty"
      />
    );
  }
  render(<Harness />);
  for (const header of screen.getAllByRole("columnheader")) {
    fireEvent.click(within(header).getByRole("button"));
    expect(body().at(-1)).toHaveTextContent("25 others");
  }
  const cells = () =>
    within(body().at(-1) ?? document.body).getAllByRole("cell");
  expect(cells()[1]).toBeEmptyDOMElement();
  expect(cells()[6]).toBeEmptyDOMElement();
  expect(cells()[2]).toHaveTextContent("90.0");
  act(() => update());
  expect(cells()[2]).toHaveTextContent("80.0");
  expect(screen.getAllByRole("table")).toHaveLength(1);
  expect(
    new Set(table.mock.results.map((result) => result.value.store)).size
  ).toBe(1);
  table.mockClear();
});
