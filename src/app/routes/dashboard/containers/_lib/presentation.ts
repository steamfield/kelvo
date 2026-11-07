import type {
  CommandError,
  DockerContainer,
  DockerHealth,
  DockerLifecycle,
  DockerStatus,
} from "@core/generated/bindings";
import type { ColumnSort } from "~/components/data-table";
export const statusLabel: Record<DockerStatus, string> = {
  disabled: "Monitoring off",
  connecting: "Connecting",
  connected: "Connected",
  idle_suspended: "Paused · no running containers",
  paused: "Paused",
  unavailable: "Unavailable",
  permission_denied: "Socket permission denied",
  incompatible: "Incompatible engine API",
};
export const lifecycleLabel: Record<DockerLifecycle, string> = {
  created: "Created",
  running: "Running",
  paused: "Paused",
  restarting: "Restarting",
  exited: "Exited",
  removing: "Removing",
  dead: "Dead",
  removed: "Removed",
  unknown: "Unknown",
};
export const healthLabel: Record<DockerHealth, string> = {
  no_check: "No check",
  starting: "Starting",
  healthy: "Healthy",
  unhealthy: "Unhealthy",
  unknown: "Unknown",
};
export const CONTAINER_SORT_COLUMNS = [
  "cpu",
  "memory",
  "name",
  "restarts",
  "network",
  "io",
  "state",
  "image",
  "ports",
] as const;
export type ContainerSortColumn = (typeof CONTAINER_SORT_COLUMNS)[number];
export type ContainerSort = ColumnSort<ContainerSortColumn>;
export const DEFAULT_CONTAINER_SORT: ContainerSort = { by: "cpu", dir: "desc" };
const TEXT_COLUMNS: readonly ContainerSortColumn[] = [
  "name",
  "state",
  "image",
  "ports",
];
export const containerSortDescFirst = (by: ContainerSortColumn) =>
  !TEXT_COLUMNS.includes(by);
export const containerFirstDir = (by: ContainerSortColumn) =>
  containerSortDescFirst(by) ? "desc" : "asc";
export const isContainerSortColumn = (
  value: string
): value is ContainerSortColumn =>
  CONTAINER_SORT_COLUMNS.some((column) => column === value);
export const isLiveLifecycle = (row: DockerContainer) =>
  row.lifecycle === "running" ||
  row.lifecycle === "paused" ||
  row.lifecycle === "restarting";
export const containerPath = (engine: string, container: string) =>
  `/dashboard/containers/${encodeURIComponent(engine)}/${encodeURIComponent(container)}`;
export function cpuLeader(rows: readonly DockerContainer[]) {
  return rows.reduce<DockerContainer | undefined>((leader, row) => {
    if (!row.monitored || row.metrics.cpu_pct === null) return leader;
    if (!leader || row.metrics.cpu_pct > (leader.metrics.cpu_pct ?? -1))
      return row;
    return row.metrics.cpu_pct === leader.metrics.cpu_pct &&
      row.id.localeCompare(leader.id) < 0
      ? row
      : leader;
  }, undefined);
}
export function filterContainers(
  rows: readonly DockerContainer[],
  query: string,
  state: string
) {
  const needle = query.trim().toLocaleLowerCase();
  return rows.filter(
    (row) =>
      (state === "all" || row.lifecycle === state) &&
      `${row.name}\n${row.image}\n${row.project ?? ""}`
        .toLocaleLowerCase()
        .includes(needle)
  );
}

/** Keep bounded, actionable errors supplied by Rust visible at the UI seam. */
export function dockerErrorMessage(error: CommandError): string {
  return "message" in error ? error.message : error.kind.replaceAll("_", " ");
}
