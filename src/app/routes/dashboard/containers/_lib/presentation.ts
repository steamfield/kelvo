import type {
  CommandError,
  DockerContainer,
  DockerHealth,
  DockerLifecycle,
  DockerStatus,
} from "@core/generated/bindings";
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
export type ContainerSortColumn =
  | "cpu"
  | "memory"
  | "name"
  | "restarts"
  | "network"
  | "io"
  | "state"
  | "image"
  | "ports";
export interface ContainerSort {
  by: ContainerSortColumn;
  dir: "asc" | "desc";
}
export const DEFAULT_CONTAINER_SORT: ContainerSort = { by: "cpu", dir: "desc" };
export const containerFirstDir = (by: ContainerSortColumn) =>
  by === "name" || by === "state" ? "asc" : "desc";
export const isContainerSortColumn = (
  value: string
): value is ContainerSortColumn =>
  [
    "cpu",
    "memory",
    "name",
    "restarts",
    "network",
    "io",
    "state",
    "image",
    "ports",
  ].includes(value);
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
