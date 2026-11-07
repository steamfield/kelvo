import type { SpaceFile, SpaceNode } from "@core/generated/bindings";
import { act, fireEvent, screen, within } from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { useState } from "react";
import { LargestFilesTable, SpaceTable } from "./space-table";

const node = (
  id: number | null,
  name: string,
  bytes: number | null,
  items: number | null
): SpaceNode => ({
  id,
  name,
  bytes,
  items,
  kind: id === null ? "smaller" : "folder",
  done: true,
  openable: id !== null,
  children: [],
});
const rows = [
  node(9, "proc9", 20, 1),
  node(10, "Proc10", 10, 2),
  node(11, "Waiting", null, null),
  node(null, "Smaller", 100, 100),
  { ...node(null, "Volume", 999, 999), kind: "volume" as const },
  { ...node(null, "Other", 999, 999), kind: "other" as const },
];
const actions = { reveal: null, copyPath: () => {} };
const names = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map(
      (r) =>
        within(r).getAllByRole("cell")[0]?.querySelector("[title]")?.textContent
    );

it.each([
  [
    "Name",
    ["Proc10", "proc9", "Volume", "Waiting"],
    ["Waiting", "Volume", "proc9", "Proc10"],
    "ascending",
  ],
  [
    "Size",
    ["Volume", "proc9", "Proc10", "Waiting"],
    ["Proc10", "proc9", "Volume", "Waiting"],
    "descending",
  ],
  [
    "Items",
    ["Volume", "Proc10", "proc9", "Waiting"],
    ["proc9", "Proc10", "Volume", "Waiting"],
    "descending",
  ],
])(
  "sorts folder %s both ways with volumes, keeps gaps and remainders last",
  (label, first, second, direction) => {
    renderWithProviders(
      <SpaceTable
        rows={rows}
        total={2000}
        hovered={null}
        onHover={() => {}}
        onOpen={() => {}}
        onUp={() => {}}
        actions={actions}
      />
    );
    expect(names()).toEqual([
      "proc9",
      "Proc10",
      "Waiting",
      "Volume",
      "Smaller",
      "Other",
    ]);
    const header = () =>
      screen.getByRole("columnheader", { name: new RegExp(label) });
    fireEvent.click(within(header()).getByRole("button"));
    expect(header()).toHaveAttribute("aria-sort", direction);
    expect(names()).toEqual([...first, "Smaller", "Other"]);
    fireEvent.click(within(header()).getByRole("button"));
    expect(header()).toHaveAttribute(
      "aria-sort",
      direction === "ascending" ? "descending" : "ascending"
    );
    expect(names()).toEqual([...second, "Smaller", "Other"]);
    expect(
      within(screen.getByRole("columnheader", { name: "Share" })).queryByRole(
        "button"
      )
    ).toBeNull();
  }
);

it("moves keyboard focus in displayed order and opens the focused folder", () => {
  const opened: string[] = [];
  renderWithProviders(
    <SpaceTable
      rows={rows}
      total={2000}
      hovered={null}
      onHover={() => {}}
      onOpen={(n) => opened.push(n.name)}
      onUp={() => {}}
      actions={actions}
    />
  );
  fireEvent.click(screen.getByRole("button", { name: "Name" }));
  act(() => screen.getByRole("row", { name: /^Proc10,/ }).focus());
  fireEvent.keyDown(document.activeElement ?? document.body, {
    key: "ArrowDown",
  });
  expect(screen.getByRole("row", { name: /^proc9,/ })).toHaveFocus();
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Enter" });
  expect(opened).toEqual(["proc9"]);
  fireEvent.keyDown(document.activeElement ?? document.body, {
    key: "ArrowUp",
  });
  expect(screen.getByRole("row", { name: /^Proc10,/ })).toHaveFocus();
});

const files: SpaceFile[] = [
  { index: 9, name: "proc9", parent: "z/folder", bytes: 20 },
  { index: 10, name: "Proc10", parent: "A/folder", bytes: 10 },
  { index: 11, name: "Waiting", parent: "m/folder", bytes: null },
];
it("breaks equal largest-file sizes and gap pairs by numeric index in both directions", () => {
  const tied = [
    { index: 10, name: "ten", parent: "same", bytes: 20 },
    { index: 2, name: "two", parent: "same", bytes: 20 },
    { index: 11, name: "gap eleven", parent: "same", bytes: null },
    { index: 3, name: "gap three", parent: "same", bytes: null },
  ];
  let update = () => {};
  function Harness() {
    const [files, setFiles] = useState(tied);
    update = () => setFiles([...tied].reverse());
    return <LargestFilesTable files={files} actions={actions} />;
  }
  renderWithProviders(<Harness />);
  const expected = ["two", "ten", "gap three", "gap eleven"];
  fireEvent.click(
    within(screen.getByRole("columnheader", { name: "Size" })).getByRole(
      "button"
    )
  );
  expect(names()).toEqual(expected);
  act(() => update());
  expect(names()).toEqual(expected);
  fireEvent.click(
    within(screen.getByRole("columnheader", { name: /Size/ })).getByRole(
      "button"
    )
  );
  expect(names()).toEqual(expected);
});
it.each([
  [
    "Name",
    ["Proc10", "proc9", "Waiting"],
    ["Waiting", "proc9", "Proc10"],
    "ascending",
  ],
  [
    "Folder",
    ["Proc10", "Waiting", "proc9"],
    ["proc9", "Waiting", "Proc10"],
    "ascending",
  ],
  [
    "Size",
    ["proc9", "Proc10", "Waiting"],
    ["Proc10", "proc9", "Waiting"],
    "descending",
  ],
])(
  "sorts largest-file %s both ways, with size gaps last",
  (label, first, second, direction) => {
    renderWithProviders(<LargestFilesTable files={files} actions={actions} />);
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
  }
);
