import { fireEvent, render, screen, within } from "@testing-library/react";
import { type VolumeRow, VolumeTable } from "./volume-table";

const rows: VolumeRow[] = [
  { id: "z", name: "proc9", usedBytes: 20, freeBytes: 1, totalBytes: 30 },
  { id: "a", name: "Proc10", usedBytes: 10, freeBytes: 2, totalBytes: 40 },
  {
    id: "gap",
    name: "Waiting",
    usedBytes: null,
    freeBytes: null,
    totalBytes: null,
  },
];
const names = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map((r) => within(r).getAllByRole("cell")[0]?.textContent);

it.each([
  [
    "Volume",
    ["Proc10", "proc9", "Waiting"],
    ["Waiting", "proc9", "Proc10"],
    "ascending",
  ],
  [
    "Used",
    ["proc9", "Proc10", "Waiting"],
    ["Proc10", "proc9", "Waiting"],
    "descending",
  ],
  [
    "Free",
    ["Proc10", "proc9", "Waiting"],
    ["proc9", "Proc10", "Waiting"],
    "descending",
  ],
  [
    "Size",
    ["Proc10", "proc9", "Waiting"],
    ["proc9", "Proc10", "Waiting"],
    "descending",
  ],
])(
  "sorts %s both ways, keeping numeric gaps last",
  (label, first, second, direction) => {
    render(
      <VolumeTable
        rows={rows}
        units="GB"
        action={(r) => <button type="button">Scan {r.name}</button>}
      />
    );
    expect(names()).toEqual(["proc9", "Proc10", "Waiting"]);
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
    for (const name of ["Usage", "Actions"])
      expect(
        within(screen.getByRole("columnheader", { name })).queryByRole("button")
      ).toBeNull();
  }
);
