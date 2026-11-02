import type { RemoteHostRecord } from "@core/generated/bindings";
import { hostKeys } from "@core/query-keys";
import { remoteErrorText } from "@core/remote-errors";
import { connectionLabel } from "@core/remote-hosts";
import { useQueryClient } from "@tanstack/react-query";
import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { Button } from "~/components/ui/button";
import { useTransport } from "~/lib/transport-context";

function connectionCopy({ config, status }: RemoteHostRecord) {
  if (status.identity_changed !== null)
    return {
      title: `Check ${config.name}’s identity`,
      message:
        "A different machine answered. Check this host in Hosts before reconnecting.",
    };
  if (status.state === "disabled")
    return {
      title: `${config.name} is disconnected`,
      message:
        "Automatic connection is turned off. You can enable it in Hosts.",
    };
  return {
    title:
      status.state === "connecting" && !status.error
        ? `Connecting to ${config.name}…`
        : `Can’t connect to ${config.name}`,
    message:
      "Live readings are unavailable. StatGlass will keep trying. You can still browse saved history.",
  };
}

/** Connection failures are recoverable states, not render errors. */
export function HostConnectionNotice({ remote }: { remote: RemoteHostRecord }) {
  const transport = useTransport();
  const cache = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [retryError, setRetryError] = useState<string | null>(null);
  const { status, config } = remote;
  const connected = status.state === "connected";
  const changed = status.identity_changed !== null;
  const copy = connectionCopy(remote);
  const retry = async () => {
    setBusy(true);
    setRetryError(null);
    try {
      const result = await transport.retryRemoteHost(config.id);
      if (result.status === "error")
        setRetryError(remoteErrorText(result.error));
      await cache.invalidateQueries({ queryKey: hostKeys.remote() });
    } catch (error) {
      console.error("[remote] retry failed", { hostId: config.id, error });
      setRetryError("The retry couldn’t be started. Try again in a moment.");
    } finally {
      setBusy(false);
    }
  };
  if (connected)
    return (
      <p
        role="status"
        className="mb-4 border-border-subtle border-b pb-3 text-[12px] text-muted-foreground"
      >
        {config.name} · {connectionLabel(remote)}
        {Math.abs(status.clock_offset_ms ?? 0) >= 30000 &&
          " · chart times follow this Mac’s clock"}
      </p>
    );
  return (
    <section
      aria-label="Host connection"
      className="mb-5 border-border-subtle border-b pb-4"
    >
      <div className="flex items-start gap-3">
        <TriangleAlert
          aria-hidden
          className="mt-0.5 size-4 shrink-0 text-muted-foreground"
        />
        <div className="min-w-0 flex-1 space-y-2">
          <div role="status">
            <h2 className="font-[590] text-[14px]">{copy.title}</h2>
            <p className="mt-1 text-[12px] text-muted-foreground">
              {copy.message}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {config.enabled && !changed && (
              <Button
                size="sm"
                variant="outline"
                disabled={busy}
                onClick={() => void retry()}
              >
                {busy ? "Retrying…" : "Retry connection"}
              </Button>
            )}
            <Button size="sm" variant="ghost" asChild>
              <Link to="/dashboard/hosts">Open Hosts</Link>
            </Button>
          </div>
          {retryError && (
            <p role="alert" className="text-[12px] text-destructive">
              {retryError}
            </p>
          )}
          {status.error && (
            <details className="text-[12px] text-muted-foreground">
              <summary className="w-fit cursor-pointer hover:text-foreground">
                Technical details
              </summary>
              <p className="wrap-anywhere mt-2 rounded-md border border-border-subtle bg-muted/30 p-3">
                {status.error}
              </p>
            </details>
          )}
        </div>
      </div>
    </section>
  );
}
