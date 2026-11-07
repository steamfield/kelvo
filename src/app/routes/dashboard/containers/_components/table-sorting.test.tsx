import type {
  DockerContainer,
  DockerStorageResource,
} from "@core/generated/bindings";
import { mockContainer, mockDockerSnapshot } from "@core/mock/docker";
import { mockStorageSnapshot } from "@core/mock/docker-storage";
import { createMockTransport } from "@core/mock-transport";
import { seriesKey } from "@core/series-key";
import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { useState } from "react";
import { useDocker } from "../_hooks/use-docker";
import type { CgroupColumn, CgroupRow, CgroupSort } from "../_lib/cgroups";
import type { ContainerSort } from "../_lib/presentation";
import ContainersRoute from "../route";
import { CgroupContainers } from "./cgroup-containers";
import { CgroupTable } from "./cgroup-table";
import { ContainerTable } from "./container-table";

function clickHeader(label: string, direction: "ascending" | "descending") {
  const head = screen.getByRole("columnheader", { name: label });
  fireEvent.click(within(head).getByRole("button"));
  expect(screen.getByRole("columnheader", { name: label })).toHaveAttribute(
    "aria-sort",
    direction
  );
}

const container = (id: string, value: number): DockerContainer => ({
  ...mockContainer(value, 1000),
  id,
  name: id,
  project: null,
  lifecycle: "running",
  image: id,
  ports: [String(value)],
  restart_count: value,
  metrics: {
    cpu_pct: value,
    working_set_bytes: value,
    raw_memory_bytes: value,
    effective_limit_bytes: null,
    network_rx_bps: value,
    network_tx_bps: value,
    block_read_bps: value,
    block_write_bps: value,
  },
});
const low = container("alpha", 1);
const high = { ...container("zulu", 2), lifecycle: "paused" as const };
const missing = {
  ...container("missing", 3),
  lifecycle: "exited" as const,
  restart_count: null,
  ports: [],
};
const extras = ["io", "image", "ports", "restarts", "pids"];
function ContainerHarness({
  historical = false,
  initial = { by: "cpu", dir: "desc" },
}: {
  historical?: boolean;
  initial?: ContainerSort;
}) {
  const { snapshot } = useDocker();
  const [sort, setSort] = useState<ContainerSort>(initial);
  return (
    <ContainerTable
      rows={snapshot?.containers ?? []}
      sort={sort}
      onSort={setSort}
      grouped={false}
      onCollapse={() => {}}
      columns={extras}
      historical={historical}
    />
  );
}
function containerSetup(rows = [missing, high, low]) {
  const transport = createMockTransport({
    scenarios: ["docker-11"],
    historyRows: 1,
  });
  let current = rows;
  const subscribe = transport.dockerSubscribe;
  transport.dockerSubscribe = (callback) =>
    subscribe((snapshot) => callback({ ...snapshot, containers: current }));
  return {
    transport,
    push: async (rows: DockerContainer[]) => {
      current = rows;
      await act(async () => transport.tick());
    },
  };
}
const containerNames = () =>
  within(screen.getByRole("table"))
    .getAllByRole("link")
    .map((node) => node.textContent);

it.each([
  [
    "Name / project",
    "ascending",
    ["alpha", "missing", "zulu"],
    ["zulu", "missing", "alpha"],
  ],
  [
    "State",
    "ascending",
    ["missing", "zulu", "alpha"],
    ["alpha", "zulu", "missing"],
  ],
  [
    "CPU",
    "descending",
    ["zulu", "alpha", "missing"],
    ["alpha", "zulu", "missing"],
  ],
  [
    "Memory",
    "descending",
    ["zulu", "alpha", "missing"],
    ["alpha", "zulu", "missing"],
  ],
  [
    "Net",
    "descending",
    ["zulu", "alpha", "missing"],
    ["alpha", "zulu", "missing"],
  ],
  [
    "Block I/O",
    "descending",
    ["zulu", "alpha", "missing"],
    ["alpha", "zulu", "missing"],
  ],
  [
    "Image",
    "ascending",
    ["alpha", "missing", "zulu"],
    ["zulu", "missing", "alpha"],
  ],
  [
    "Ports",
    "ascending",
    ["alpha", "zulu", "missing"],
    ["zulu", "alpha", "missing"],
  ],
  [
    "Restarts",
    "descending",
    ["zulu", "alpha", "missing"],
    ["alpha", "zulu", "missing"],
  ],
] as const)(
  "container %s sorts through the header in both directions",
  async (label, first, forward, backward) => {
    const setup = containerSetup();
    const result = renderWithProviders(
      <ContainerHarness initial={{ by: "state", dir: "desc" }} />,
      { transport: setup.transport }
    );
    await screen.findByRole("link", { name: "alpha" });
    // State starts sorted descending so its first click is ascending.
    clickHeader(label, first);
    expect(containerNames()).toEqual(forward);
    clickHeader(label, first === "ascending" ? "descending" : "ascending");
    expect(containerNames()).toEqual(backward);
    expect(
      screen.getByRole("link", { name: "missing" }).closest("tr")
    ).not.toHaveTextContent(/\b0(?:\.0)?%/);
    for (const name of ["Chart color", "Action", "PIDs"])
      expect(
        within(screen.getByRole("columnheader", { name })).queryByRole("button")
      ).toBeNull();
    result.unmount();
    setup.transport.dispose();
  }
);

it("container focus holds across a pushed snapshot, and a header click applies immediately", async () => {
  const setup = containerSetup([high, low]);
  const result = renderWithProviders(<ContainerHarness />, {
    transport: setup.transport,
  });
  await screen.findByRole("link", { name: "zulu" });
  act(() => screen.getByRole("link", { name: "alpha" }).focus());
  await setup.push([
    { ...low, metrics: { ...low.metrics, cpu_pct: 99 } },
    high,
  ]);
  expect(containerNames()).toEqual(["zulu", "alpha"]);
  expect(screen.getByText("99%")).toBeVisible();
  clickHeader("Name / project", "ascending");
  expect(containerNames()).toEqual(["alpha", "zulu"]);
  fireEvent.blur(screen.getByRole("link", { name: "alpha" }), {
    relatedTarget: document.body,
  });
  result.unmount();
  setup.transport.dispose();
});

it("container ties and gap pairs keep identity order across reversed input", async () => {
  const a = { ...low, metrics: { ...low.metrics, cpu_pct: null } };
  const b = { ...high, metrics: { ...low.metrics, cpu_pct: null } };
  const setup = containerSetup([
    b,
    a,
    { ...low, id: "measured", name: "measured" },
  ]);
  const result = renderWithProviders(<ContainerHarness />, {
    transport: setup.transport,
  });
  await screen.findByRole("link", { name: "measured" });
  expect(containerNames()).toEqual(["measured", "alpha", "zulu"]);
  clickHeader("CPU", "ascending");
  expect(containerNames()).toEqual(["measured", "alpha", "zulu"]);
  await setup.push([a, b, { ...low, id: "measured", name: "measured" }]);
  expect(containerNames()).toEqual(["measured", "alpha", "zulu"]);
  await setup.push([high, { ...low, metrics: high.metrics }]);
  expect(containerNames()).toEqual(["alpha", "zulu"]);
  clickHeader("CPU", "descending");
  expect(containerNames()).toEqual(["alpha", "zulu"]);
  result.unmount();
  setup.transport.dispose();
});

it("historical Net falls back to CPU with matching head and body cells", async () => {
  const setup = containerSetup([high, low]);
  const result = renderWithProviders(
    <ContainerHarness historical initial={{ by: "network", dir: "asc" }} />,
    { transport: setup.transport }
  );
  await screen.findByRole("link", { name: "alpha" });
  expect(screen.queryByRole("columnheader", { name: "Net" })).toBeNull();
  expect(
    screen.getByRole("columnheader", { name: "Peak CPU" })
  ).toHaveAttribute("aria-sort", "descending");
  expect(containerNames()).toEqual(["zulu", "alpha"]);
  expect(screen.getAllByRole("columnheader")).toHaveLength(
    within(
      screen.getByRole("link", { name: "alpha" }).closest("tr") ?? document.body
    ).getAllByRole("cell").length
  );
  result.unmount();
  setup.transport.dispose();
});

it("removing the sorted optional column falls back to CPU", async () => {
  const setup = containerSetup([high, low]);
  const result = renderWithProviders(<ContainersRoute />, {
    transport: setup.transport,
    route: "/dashboard/containers?columns=io&sort=io&dir=asc",
  });
  await screen.findByRole("link", { name: "alpha" });
  expect(containerNames()).toEqual(["alpha", "zulu"]);
  await act(async () => {
    await result.router.navigate("/dashboard/containers?sort=io&dir=asc");
  });
  expect(screen.queryByRole("columnheader", { name: "Block I/O" })).toBeNull();
  expect(screen.getByRole("columnheader", { name: "CPU" })).toHaveAttribute(
    "aria-sort",
    "descending"
  );
  expect(containerNames()).toEqual(["zulu", "alpha"]);
  result.unmount();
  setup.transport.dispose();
});

const cgroup = (name: string, value: number | null): CgroupRow => ({
  id: name,
  name,
  kind: name === "alpha" ? "docker" : "service",
  cpu: value,
  mem: value,
  memMax: null,
  ioRead: value,
  ioWrite: value,
  memPsi: value,
});
const cgroupColumns: CgroupColumn[] = [
  "name",
  "kind",
  "cpu",
  "mem",
  "ioRead",
  "ioWrite",
  "memPsi",
];
const cgroupRows = [
  cgroup("missing", null),
  cgroup("zulu", 2),
  cgroup("alpha", 1),
];
function CgroupHarness({ rows = cgroupRows }: { rows?: CgroupRow[] }) {
  const [sort, setSort] = useState<CgroupSort>({ by: "cpu", dir: "asc" });
  return (
    <CgroupTable
      rows={rows}
      other={{ label: "25 others", row: cgroup("_other", 999) }}
      columns={cgroupColumns}
      sort={sort}
      onSort={setSort}
      bytes="GB"
      rate="MBps"
      empty="No groups"
    />
  );
}

it.each([
  "Name",
  "Kind",
  "% CPU",
  "Memory",
  "Disk read",
  "Disk write",
  "Memory pressure",
])("cgroup %s sorts both ways and pins others last", (label) => {
  const result = renderWithProviders(<CgroupHarness />);
  const text = label === "Name" || label === "Kind";
  clickHeader(label, text ? "ascending" : "descending");
  const rows = within(screen.getByRole("table")).getAllByRole("row").slice(1);
  expect(rows[0]).toHaveTextContent(text ? "alpha" : "zulu");
  expect(rows.at(-1)).toHaveTextContent("25 others");
  clickHeader(label, text ? "descending" : "ascending");
  const reversed = within(screen.getByRole("table"))
    .getAllByRole("row")
    .slice(1);
  expect(reversed[0]).toHaveTextContent(
    label === "Kind" ? "missing" : text ? "zulu" : "alpha"
  );
  expect(reversed.at(-1)).toHaveTextContent("25 others");
  if (!text) expect(reversed.at(-2)).toHaveTextContent("missing");
  result.unmount();
  result.transport.dispose();
});

it("cgroup hover holds a pushed frame, applies header changes immediately and breaks ties by name", async () => {
  const result = renderWithProviders(<CgroupContainers />);
  await waitFor(() => expect(result.transport.subscriberCount()).toBe(1));
  act(() => {
    result.transport.push({
      kind: "layout",
      layout_no: 90,
      series: [
        seriesKey("cgroup.cpu", { kind: "docker", name: "zulu" }),
        seriesKey("cgroup.cpu", { kind: "docker", name: "alpha" }),
      ],
      kinds: ["mean", "mean"],
    });
    result.transport.push({
      kind: "frame",
      layout_no: 90,
      ts_ms: 1800000000000,
      timeline: 1,
      values: [2, 1],
      held: [2, 1],
    });
  });
  const firstName = () =>
    within(screen.getByRole("table")).getAllByRole("row")[1];
  expect(firstName()).toHaveTextContent("zulu");
  fireEvent.pointerEnter(
    screen.getByRole("table").parentElement ?? document.body
  );
  act(() =>
    result.transport.push({
      kind: "frame",
      layout_no: 90,
      ts_ms: 1800000001000,
      timeline: 1,
      values: [0.5, 9],
      held: [0.5, 9],
    })
  );
  expect(firstName()).toHaveTextContent("zulu");
  clickHeader("Name", "ascending");
  expect(firstName()).toHaveTextContent("alpha");
  clickHeader("Name", "descending");
  expect(firstName()).toHaveTextContent("zulu");
  fireEvent.pointerLeave(
    screen.getByRole("table").parentElement ?? document.body
  );
  clickHeader("% CPU", "descending");
  expect(firstName()).toHaveTextContent("alpha");
  act(() =>
    result.transport.push({
      kind: "frame",
      layout_no: 90,
      ts_ms: 1800000002000,
      timeline: 1,
      values: [1, 1],
      held: [1, 1],
    })
  );
  expect(firstName()).toHaveTextContent("alpha");
  clickHeader("% CPU", "ascending");
  expect(firstName()).toHaveTextContent("alpha");
  result.unmount();
  result.transport.dispose();
});

async function storageSetup(resources: DockerStorageResource[]) {
  const transport = createMockTransport({
    scenarios: ["docker-32"],
    historyRows: 1,
  });
  const snapshot = mockStorageSnapshot(mockDockerSnapshot(32, 1000), 1000);
  transport.dockerStorageScan = async () => ({
    status: "ok",
    data: {
      snapshot: { ...snapshot, resources },
      error: null,
      latest_operation_id: null,
    },
  });
  const result = renderWithProviders(<ContainersRoute />, {
    transport,
    route: "/dashboard/containers?view=storage",
  });
  await screen.findByRole("table", { name: "Storage resources" });
  return result;
}
function storageFixture(id: string, value: number): DockerStorageResource {
  const base = mockStorageSnapshot(mockDockerSnapshot(1, 1000), 1000)
    .resources[0];
  if (!base) throw new Error("Missing storage fixture");
  return {
    ...base,
    id,
    name: id,
    kind: "volume",
    size_bytes: value,
    age_ms: value,
    references: Array.from({ length: value }, (_, index) => String(index)),
    can_delete: true,
    reason: null,
  };
}
const storageNames = () =>
  within(screen.getByRole("table", { name: "Storage resources" }))
    .getAllByRole("checkbox")
    .map((node) => node.getAttribute("aria-label")?.replace("Select ", ""));

it.each([
  "Resource / type",
  "Size",
  "References",
  "Age evidence",
  "Eligibility",
])("storage %s sorts both ways, with gaps last", async (label) => {
  const result = await storageSetup([
    {
      ...storageFixture("missing", 3),
      size_bytes: null,
      age_ms: null,
      can_delete: false,
      reason: "Z reason",
    },
    storageFixture("zulu", 2),
    storageFixture("alpha", 1),
  ]);
  // Start Size ascending so its next click exercises the first descending order.
  if (label === "Size") clickHeader("Size", "ascending");
  const text = label === "Resource / type" || label === "Eligibility";
  clickHeader(label, text ? "ascending" : "descending");
  expect(storageNames()).toEqual(
    label === "Resource / type"
      ? ["alpha", "missing", "zulu"]
      : label === "Eligibility"
        ? ["alpha", "zulu", "missing"]
        : label === "References"
          ? ["missing", "zulu", "alpha"]
          : ["zulu", "alpha", "missing"]
  );
  clickHeader(label, text ? "descending" : "ascending");
  expect(storageNames()).toEqual(
    label === "Resource / type"
      ? ["zulu", "missing", "alpha"]
      : label === "Eligibility"
        ? ["missing", "alpha", "zulu"]
        : ["alpha", "zulu", "missing"]
  );
  result.unmount();
  result.transport.dispose();
});

it("storage sorts the full inventory before paging and breaks ties by kind:id", async () => {
  const resources = Array.from({ length: 35 }, (_, index) =>
    storageFixture(`resource-${String(index).padStart(2, "0")}`, index)
  );
  const result = await storageSetup(resources);
  expect(storageNames()[0]).toBe("resource-34");
  fireEvent.click(screen.getByRole("button", { name: "Next resources" }));
  expect(storageNames()).toEqual(["resource-02", "resource-01", "resource-00"]);
  clickHeader("Size", "ascending");
  expect(storageNames()).toEqual(["resource-32", "resource-33", "resource-34"]);
  result.unmount();
  result.transport.dispose();
  const tied = await storageSetup([
    { ...storageFixture("zulu", 1), kind: "image" },
    storageFixture("alpha", 1),
  ]);
  expect(storageNames()).toEqual(["zulu", "alpha"]);
  clickHeader("Size", "ascending");
  expect(storageNames()).toEqual(["zulu", "alpha"]);
  tied.unmount();
  tied.transport.dispose();
});
