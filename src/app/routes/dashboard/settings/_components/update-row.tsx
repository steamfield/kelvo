import type { UpdateStatus } from "@core/generated/bindings";
import { useState } from "react";
import { SettingsRow } from "~/components/settings-row";
import { Button } from "~/components/ui/button";
import { useLocalUpdateStatus } from "~/hooks/use-local-update-status";
import { useTransport } from "~/lib/transport-context";
import { version } from "../../../../../../package.json";

function updateMessage(status: UpdateStatus): string {
  switch (status.kind) {
    case "idle":
      return "StatGlass checks for releases according to your update settings.";
    case "not_configured":
      return "Use a release build to check for updates.";
    case "checking":
      return "Checking GitHub Releases…";
    case "up_to_date":
      return "You’re running the latest version.";
    case "available":
      return `StatGlass ${status.version} is available. Installing restarts StatGlass; your history and settings are kept.`;
    case "downloading":
      return status.percent === null
        ? "Downloading update…"
        : `Downloading update… ${status.percent}%`;
    case "downloaded":
      return `StatGlass ${status.version} is downloaded and ready to install.`;
    case "waiting":
      return `StatGlass ${status.version} is ready. ${status.reason}`;
    case "compatibility_blocked":
      return `StatGlass ${status.version} is waiting: ${status.message}`;
    case "needs_authorization":
      return `StatGlass ${status.version} needs your permission to install. Choose Install and restart.`;
    case "installing":
      return "Installing update…";
    case "ready":
      return "Update installed. Restarting StatGlass…";
    case "failed":
      return `Couldn’t update: ${status.message}`;
  }
}

export function UpdateRow() {
  const transport = useTransport();
  const [status, setStatus] = useLocalUpdateStatus();
  const [pending, setPending] = useState(false);

  const act = async () => {
    setPending(true);
    try {
      const result =
        status.kind === "available" ||
        status.kind === "downloaded" ||
        status.kind === "waiting" ||
        status.kind === "needs_authorization"
          ? await transport.installUpdate(status.version)
          : await transport.checkForUpdates();
      if (result.status === "error") {
        setStatus({
          kind: "failed",
          message:
            "message" in result.error
              ? result.error.message
              : "Please try again.",
        });
      } else if (result.data) {
        setStatus(result.data);
      }
    } catch (error) {
      setStatus({ kind: "failed", message: String(error) });
    } finally {
      setPending(false);
    }
  };
  const busy =
    pending ||
    ["checking", "downloading", "installing", "ready"].includes(status.kind);
  return (
    <SettingsRow
      label="Version"
      sub={
        <>
          <span role="status">{updateMessage(status)}</span>
          {status.kind === "available" && status.notes && (
            <details className="mt-2">
              <summary className="cursor-pointer">Release notes</summary>
              <span className="mt-2 block max-h-40 overflow-auto whitespace-pre-wrap">
                {status.notes}
              </span>
            </details>
          )}
        </>
      }
    >
      <span className="figures text-[12px] text-fg-subtle">{version}</span>
      <Button
        variant="outline"
        size="sm"
        disabled={busy || status.kind === "not_configured"}
        onClick={act}
      >
        {["available", "downloaded", "waiting", "needs_authorization"].includes(
          status.kind
        )
          ? "Install and restart"
          : busy
            ? "Please wait…"
            : "Check now"}
      </Button>
    </SettingsRow>
  );
}
