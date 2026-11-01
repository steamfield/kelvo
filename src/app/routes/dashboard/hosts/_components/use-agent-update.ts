import type { RemoteHostRecord } from "@core/generated/bindings";
import { hostKeys } from "@core/query-keys";
import { remoteErrorText } from "@core/remote-errors";
import {
  useMutation,
  useMutationState,
  useQueryClient,
} from "@tanstack/react-query";
import { useTransport } from "~/lib/transport-context";

const AGENT_UPDATE = ["agent-update"] as const;

/** Linux hosts behind this Mac that "Update agent" can push to over SSH. */
export function pushable(remote: RemoteHostRecord): boolean {
  const update = remote.status.agent_update;
  return (
    (update.kind === "available" || update.kind === "required") &&
    update.via === "push" &&
    remote.config.enabled &&
    remote.status.identity_changed === null
  );
}

/**
 * One mutation per request, keyed by the host id, so a row's button and
 * "Update all" show the same in-flight state. Resolves to an error message,
 * or null once the host accepted the request.
 */
export function useAgentUpdate() {
  const transport = useTransport();
  const cache = useQueryClient();
  return useMutation({
    mutationKey: AGENT_UPDATE,
    mutationFn: async (id: string): Promise<string | null> => {
      try {
        const result = await transport.updateRemoteAgent(id);
        return result.status === "error" ? remoteErrorText(result.error) : null;
      } catch (e) {
        console.error("[remote hosts] agent update failed", {
          hostId: id,
          error: e,
        });
        return String(e);
      } finally {
        await cache.invalidateQueries({ queryKey: hostKeys.remote() });
        await cache.invalidateQueries({ queryKey: hostKeys.updates(id) });
      }
    },
  });
}

/** Whether Update all can reach the host now; others are listed as skipped. */
export function reachable(remote: RemoteHostRecord): boolean {
  return (
    remote.status.state === "connected" ||
    remote.status.agent_update.kind === "required"
  );
}

/** Whether an update request for this host is in flight. */
export function useAgentUpdating(id: string): boolean {
  return useMutationState({
    filters: { mutationKey: AGENT_UPDATE, status: "pending" },
    select: (m) => m.state.variables,
  }).includes(id);
}
