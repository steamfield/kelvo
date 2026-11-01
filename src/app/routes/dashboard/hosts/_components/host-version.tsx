import type { RemoteHostRecord } from "@core/generated/bindings";
import { agentVersionLine, releaseUrl } from "@core/remote-hosts";
import { useTransport } from "~/lib/transport-context";
import { cn } from "~/lib/utils";

/** The host's Kelvo version, what this Mac can install, and that release's notes. */
export function HostVersion({
  remote,
  localTarget,
}: {
  remote: RemoteHostRecord;
  localTarget: string | null;
}) {
  const transport = useTransport();
  const line = agentVersionLine(remote, localTarget);
  if (!line) return null;
  const update = remote.status.agent_update;
  const notes =
    update.kind === "available" || update.kind === "required"
      ? releaseUrl(update.target)
      : null;
  return (
    <p
      className={cn(
        "figures mt-1 text-[12px]",
        line.tone === "destructive"
          ? "text-destructive"
          : "text-muted-foreground"
      )}
    >
      {line.text}
      {notes && (
        <>
          {" "}
          <button
            type="button"
            onClick={() =>
              void transport.openUrl(notes).catch((error: unknown) =>
                console.error("[remote hosts] opening release notes failed", {
                  url: notes,
                  error,
                })
              )
            }
            className="rounded-chip text-link outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
          >
            What’s new
          </button>
        </>
      )}
    </p>
  );
}
