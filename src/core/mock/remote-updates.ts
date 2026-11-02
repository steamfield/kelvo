import type { AgentUpdate, RemoteHostStatus } from "@core/generated/bindings";
import type { ScenarioName } from "./fixtures";
import type { MockRemotes } from "./remote";
import { APP_VERSION } from "./remote-linux";

const OLD = "0.0.9";
const behind = (via: "push" | "add_host"): AgentUpdate => ({
  kind: "available",
  installed: OLD,
  target: APP_VERSION,
  via,
});

function seed(
  remotes: MockRemotes,
  name: string,
  linux: boolean,
  status: Partial<RemoteHostStatus>
) {
  const record = remotes.addRemote(
    {
      id: `remote-${name}`,
      name,
      lan: `${name}.local`,
      tailscale: null,
      enabled: true,
      mirror_budget_mb: 100,
    },
    linux ? ["linux"] : ["no-battery", "no-fans"]
  );
  record.status = { ...record.status, ...status };
}

/** Hosts behind, ahead of, or unable to talk to this Mac's version (D-123). */
export function seedUpdateScenarios(
  remotes: MockRemotes,
  scenarios: readonly ScenarioName[]
) {
  if (scenarios.includes("host-unreachable"))
    seed(remotes, "unreachable", true, {
      state: "error",
      route: null,
      last_seen_ms: null,
      cpu_pct: null,
      memory_pct: null,
      error:
        "SSH command failed (127): /usr/local/bin/statglass-agent: No such file or directory",
      clock_offset_ms: null,
    });
  if (scenarios.includes("hosts-outdated")) {
    for (const name of ["nas", "build-box"])
      seed(remotes, name, true, {
        agent_version: OLD,
        agent_update: behind("push"),
      });
    seed(remotes, "studio", false, {
      agent_version: OLD,
      agent_update: behind("add_host"),
    });
    seed(remotes, "pi", true, {
      state: "offline",
      route: null,
      cpu_pct: null,
      memory_pct: null,
      agent_version: OLD,
      agent_update: behind("push"),
    });
  }
  if (scenarios.includes("host-incompatible"))
    seed(remotes, "legacy", true, {
      state: "error",
      route: null,
      cpu_pct: null,
      memory_pct: null,
      error: "incompatible remote: protocol 1 is below the supported range 2-3",
      agent_version: "0.0.1",
      agent_update: {
        kind: "required",
        installed: "0.0.1",
        target: APP_VERSION,
        via: "push",
      },
    });
  if (scenarios.includes("host-newer"))
    seed(remotes, "edge", true, {
      agent_version: "0.2.0",
      agent_update: { kind: "newer_than_this", installed: "0.2.0" },
    });
}
