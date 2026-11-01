import type {
  AgentUpdate,
  RemoteHostRecord,
  RemoteState,
  UpdateStatus,
} from "@core/generated/bindings";
import {
  agentVersionLine,
  hostUpdateNotice,
  localUpdateTarget,
  releaseUrl,
} from "./remote-hosts";

function host(
  agent_update: AgentUpdate,
  state: RemoteState = "connected"
): RemoteHostRecord {
  return {
    config: {
      id: "h",
      name: "box",
      lan: "box",
      tailscale: null,
      enabled: true,
      mirror_budget_mb: 100,
    },
    os: "linux",
    status: {
      state,
      route: "lan",
      last_seen_ms: null,
      error: null,
      sync: "current",
      clock_offset_ms: 0,
      cpu_pct: null,
      memory_pct: null,
      agent_version:
        "installed" in agent_update ? agent_update.installed : null,
      agent_update,
      identity_changed: null,
    },
  };
}

const behind = host({
  kind: "available",
  installed: "0.3.0",
  target: "0.3.1",
  via: "push",
});
const current = host({ kind: "up_to_date", installed: "0.3.1" });
const blocked = host({
  kind: "required",
  installed: "0.2.0",
  target: "0.3.1",
  via: "push",
});

describe("hostUpdateNotice", () => {
  it("says nothing when every host is current or unknown", () => {
    expect(
      hostUpdateNotice([current, host({ kind: "unknown" })], null, null)
    ).toBeNull();
  });

  it("counts hosts behind and keys the dismissal to the target", () => {
    const notice = hostUpdateNotice([behind, behind, current], null, null);
    expect(notice).toEqual({ kind: "hosts", count: 2, key: "hosts:0.3.1" });
    expect(hostUpdateNotice([behind], null, "hosts:0.3.1")).toBeNull();
    expect(hostUpdateNotice([behind], null, "hosts:0.3.0")).not.toBeNull();
  });

  it("puts this Mac's own pending update first", () => {
    const available: UpdateStatus = {
      kind: "available",
      version: "0.3.2",
      notes: "",
    };
    expect(hostUpdateNotice([behind], available, "hosts:0.3.1")).toEqual({
      kind: "local_first",
      version: "0.3.2",
      key: "local:0.3.2",
    });
    expect(
      hostUpdateNotice([behind], { kind: "ready", version: "0.3.2" }, null)
    ).toMatchObject({ kind: "local_restart" });
    expect(hostUpdateNotice([behind], available, "local:0.3.2")).toBeNull();
  });

  it("can't dismiss a host that can't connect", () => {
    expect(hostUpdateNotice([blocked, behind], null, "hosts:0.3.1")).toEqual({
      kind: "required",
      count: 1,
    });
  });
});

describe("localUpdateTarget", () => {
  it.each<[UpdateStatus, string | null]>([
    [{ kind: "idle" }, null],
    [{ kind: "up_to_date" }, null],
    [{ kind: "checking" }, null],
    [{ kind: "failed", message: "x" }, null],
    [{ kind: "compatibility_blocked", version: "0.4.0", message: "x" }, null],
    [{ kind: "downloading", version: "0.3.2", percent: 10 }, "0.3.2"],
    [{ kind: "needs_authorization", version: "0.3.2" }, "0.3.2"],
    [{ kind: "installing", version: "0.3.2" }, "0.3.2"],
  ])("%o → %s", (status, target) => {
    expect(localUpdateTarget(status)).toBe(target);
  });
});

describe("agentVersionLine", () => {
  it("shows the installed and installable versions", () => {
    expect(agentVersionLine(behind, null)?.text).toBe(
      "Kelvo 0.3.0 → 0.3.1 available"
    );
    expect(agentVersionLine(current, "0.3.2")?.text).toBe(
      "Kelvo 0.3.1 · 0.3.2 after updating this Mac"
    );
    expect(agentVersionLine(host({ kind: "unknown" }), null)).toBeNull();
  });

  it("marks an offline host's version as last reported", () => {
    const offline = host({ kind: "up_to_date", installed: "0.3.1" }, "offline");
    expect(agentVersionLine(offline, null)?.text).toBe(
      "Kelvo 0.3.1 (last reported)"
    );
  });

  it("flags a host that can't connect", () => {
    expect(
      agentVersionLine(
        { ...blocked, status: { ...blocked.status, state: "error" } },
        null
      )
    ).toEqual({
      text: "Kelvo 0.2.0 can’t connect to this version. Update it to 0.3.1.",
      tone: "destructive",
    });
  });

  it("points a Mac host at Add host", () => {
    const mac = host({
      kind: "available",
      installed: "0.3.0",
      target: "0.3.1",
      via: "add_host",
    });
    expect(agentVersionLine(mac, null)?.text).toContain("from Add host");
  });
});

describe("releaseUrl", () => {
  it("links only plain versions", () => {
    expect(releaseUrl("0.3.1")).toBe(
      "https://github.com/steamfield/kelvo/releases/tag/v0.3.1"
    );
    expect(releaseUrl("0.3.1/../../settings")).toBeNull();
  });
});
