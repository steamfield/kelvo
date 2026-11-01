import type { RemoteHostRecord } from "@core/generated/bindings";
import { useState } from "react";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "~/components/ui/dialog";
import { useAgentUpdate } from "./use-agent-update";

type Outcome = { id: string; result: string };

/**
 * Pushes this Mac's agent to each host in turn (the backend runs one at a
 * time). Each push restarts that host's service, so the list is confirmed
 * first; results are one list, not a toast per host (design-system.md).
 * Offline hosts are listed but not attempted.
 */
export function UpdateAllDialog({
  remotes,
  close,
}: {
  remotes: readonly RemoteHostRecord[];
  close: () => void;
}) {
  const agentUpdate = useAgentUpdate();
  const [running, setRunning] = useState(false);
  const [outcomes, setOutcomes] = useState<Outcome[] | null>(null);
  const reachable = (r: RemoteHostRecord) =>
    r.status.state === "connected" || r.status.agent_update.kind === "required";
  const run = async () => {
    setRunning(true);
    const done: Outcome[] = [];
    for (const remote of remotes) {
      const id = remote.config.id;
      if (!reachable(remote)) {
        done.push({ id, result: "Offline: not updated" });
        continue;
      }
      const error = await agentUpdate.mutateAsync(id);
      done.push({ id, result: error ?? "Update requested" });
      setOutcomes([...done]);
    }
    setOutcomes(done);
    setRunning(false);
  };
  const requested = outcomes?.filter((o) => o.result === "Update requested");
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !running) close();
      }}
    >
      <DialogContent showCloseButton={!running}>
        <DialogTitle>Update agents</DialogTitle>
        <DialogDescription>
          {outcomes && !running
            ? `Requested on ${requested?.length ?? 0} of ${remotes.length}. Each host finishes the update on its own and reconnects.`
            : "Kelvo installs this Mac’s agent on each host below. Each host’s service restarts, so its live readings pause briefly."}
        </DialogDescription>
        <ul className="m-0 max-h-60 list-none divide-y divide-border-subtle overflow-y-auto border-border-subtle border-y p-0 text-[13px]">
          {remotes.map((remote) => {
            const outcome = outcomes?.find((o) => o.id === remote.config.id);
            return (
              <li
                key={remote.config.id}
                className="flex justify-between gap-4 py-2"
              >
                <span className="truncate">{remote.config.name}</span>
                <span
                  className={
                    outcome && outcome.result !== "Update requested"
                      ? "text-destructive"
                      : "text-muted-foreground"
                  }
                >
                  {outcome?.result ??
                    (reachable(remote) ? "" : "Offline: not updated")}
                </span>
              </li>
            );
          })}
        </ul>
        <DialogFooter>
          {outcomes && !running ? (
            <Button onClick={close}>Done</Button>
          ) : (
            <>
              <Button variant="outline" disabled={running} onClick={close}>
                Cancel
              </Button>
              <Button disabled={running} onClick={() => void run()}>
                {running ? "Updating…" : "Update agents"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
