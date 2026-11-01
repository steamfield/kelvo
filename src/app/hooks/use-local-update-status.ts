import type { UpdateStatus } from "@core/generated/bindings";
import { useEffect, useState } from "react";
import { useTransport } from "~/lib/transport-context";

/** This Mac's own update status, kept current by `update-status-changed`. */
export function useLocalUpdateStatus() {
  const transport = useTransport();
  const [status, setStatus] = useState<UpdateStatus>({ kind: "idle" });
  useEffect(() => {
    let active = true;
    let receivedEvent = false;
    const stop = transport.onUpdateStatusChanged((next) => {
      receivedEvent = true;
      if (active) setStatus(next);
    });
    transport
      .getUpdateStatus()
      .then((next) => {
        if (active && !receivedEvent) setStatus(next);
      })
      .catch((error: unknown) => {
        if (active && !receivedEvent)
          setStatus({ kind: "failed", message: String(error) });
      });
    return () => {
      active = false;
      stop();
    };
  }, [transport]);
  return [status, setStatus] as const;
}
