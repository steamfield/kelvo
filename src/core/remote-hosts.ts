import type {
  LinuxPreparation,
  RemoteHostRecord,
  UpdateStatus,
} from "@core/generated/bindings";

export function connectionLabel(remote: RemoteHostRecord): string {
  const { status } = remote;
  if (status.state === "connected")
    return `${status.route === "lan" ? "LAN" : "Tailscale"}${status.sync === "catching_up" ? " · catching up history" : status.sync === "unavailable" ? " · history unavailable" : " · connected"}`;
  return (
    {
      connecting: "Connecting…",
      offline: "Offline",
      disabled: "Connection disabled",
      error: "Connection failed",
    }[status.state] ?? status.state
  );
}

/** What the probe found on a Linux host: "Linux 6.17 · aarch64 · systemd 256". */
export function linuxSystemLine(linux: LinuxPreparation): string {
  const kernel = linux.kernel?.match(/^\d+\.\d+/)?.[0];
  return [
    kernel ? `Linux ${kernel}` : "Linux",
    linux.arch,
    linux.systemd_version === null ? null : `systemd ${linux.systemd_version}`,
  ]
    .filter((part) => part)
    .join(" · ");
}

/** Hosts this Mac can bring up to its own version (D-123). */
export function hostsNeedingUpdate(
  remotes: readonly RemoteHostRecord[]
): RemoteHostRecord[] {
  return remotes.filter(
    (r) =>
      r.status.agent_update.kind === "available" ||
      r.status.agent_update.kind === "required"
  );
}

/**
 * The version this Mac is about to move to, when its own update is under way.
 * The updater only reports a version for a release newer than this build, so
 * no comparison is needed here.
 */
export function localUpdateTarget(status: UpdateStatus | null): string | null {
  switch (status?.kind) {
    case "available":
    case "downloading":
    case "downloaded":
    case "waiting":
    case "needs_authorization":
    case "installing":
    case "ready":
      return status.version;
    default:
      return null;
  }
}

/**
 * The row's version line. A version from an offline host is the last one it
 * reported; an incompatible host just reported it, so it stays unqualified.
 */
export function agentVersionLine(
  remote: RemoteHostRecord,
  localTarget: string | null
): { text: string; tone: "muted" | "destructive" } | null {
  const update = remote.status.agent_update;
  if (update.kind === "unknown") return null;
  const seen =
    remote.status.state !== "connected" && update.kind !== "required"
      ? " (last reported)"
      : "";
  const installed = `Kelvo ${update.installed}${seen}`;
  const later = localTarget ? ` · ${localTarget} after updating this Mac` : "";
  switch (update.kind) {
    case "up_to_date":
      return { text: `${installed}${later}`, tone: "muted" };
    case "available":
      return {
        text: `${installed} → ${update.target} available${update.via === "add_host" ? " · update it from Add host" : ""}${later}`,
        tone: "muted",
      };
    case "required":
      return {
        text: `${installed} can’t connect to this version. Update it to ${update.target}${update.via === "add_host" ? " from Add host" : ""}.`,
        tone: "destructive",
      };
    case "newer_than_this":
      return {
        text: `${installed}, newer than this Mac. Update this Mac in Settings.`,
        tone: "muted",
      };
  }
}

/** A release page link, only for a plain version the opener scope allows. */
export function releaseUrl(version: string): string | null {
  return /^\d+\.\d+\.\d+$/.test(version)
    ? `https://github.com/steamfield/kelvo/releases/tag/v${version}`
    : null;
}

export type HostUpdateNotice =
  | { kind: "required"; count: number }
  | { kind: "local_first"; version: string; key: string }
  | { kind: "local_restart"; version: string; key: string }
  | { kind: "hosts"; count: number; key: string };

/**
 * The dashboard notice (D-123). A host that can't connect always shows. When
 * this Mac has its own newer release on the way, that comes first: pushing
 * the current agent now would be repeated after the restart. Dismissing hides
 * a notice until its version changes.
 */
export function hostUpdateNotice(
  remotes: readonly RemoteHostRecord[],
  local: UpdateStatus | null,
  dismissed: string | null
): HostUpdateNotice | null {
  const behind = hostsNeedingUpdate(remotes);
  if (behind.length === 0) return null;
  const required = behind.filter(
    (r) => r.status.agent_update.kind === "required"
  ).length;
  if (required > 0) return { kind: "required", count: required };
  const first = behind[0]?.status.agent_update;
  const hostTarget = first && "target" in first ? first.target : "";
  const target = localUpdateTarget(local);
  const notice: HostUpdateNotice =
    target === null
      ? {
          kind: "hosts",
          count: behind.length,
          key: `hosts:${hostTarget}`,
        }
      : local?.kind === "installing" || local?.kind === "ready"
        ? { kind: "local_restart", version: target, key: `local:${target}` }
        : { kind: "local_first", version: target, key: `local:${target}` };
  return notice.key === dismissed ? null : notice;
}
