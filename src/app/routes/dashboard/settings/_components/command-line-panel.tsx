import type { CliLinkStatus } from "@core/generated/bindings";
import { appKeys } from "@core/query-keys";
import { unwrap } from "@core/transport";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ConfirmDialog } from "~/components/confirm-dialog";
import { SettingsPanel, SettingsRow } from "~/components/settings-row";
import { Button } from "~/components/ui/button";
import { Switch } from "~/components/ui/switch";
import { useWriteSettings } from "~/hooks/use-write-settings";
import { useTransport } from "~/lib/transport-context";

const SKILL_COMMAND = "statglass skill install";

/** `/Users/name/...` paths in `text` as a shell user writes them. */
const tilde = (text: string) => text.replace(/\/Users\/[^/\s]+\//g, "~/");

function linkMessage(link: CliLinkStatus): string {
  const path = tilde(link.link_path);
  switch (link.state) {
    case "current": {
      const dir = path.slice(0, path.lastIndexOf("/"));
      // `on_path` is this app's PATH; from Finder that is launchd's short one.
      return link.on_path
        ? `Installed at ${path}.`
        : `Installed at ${path}. If your shell can't find statglass, add ${dir} to PATH.`;
    }
    case "missing":
      return link.target === null
        ? "This build has no bundled command-line tool."
        : "Not installed.";
    case "stale":
      return `${path} points to another copy of StatGlass.`;
    case "foreign":
      return `${path} points to ${tilde(link.points_to ?? "something else")}, not StatGlass.`;
    case "blocked":
      return tilde(link.reason ?? `${path} is not a link.`);
  }
}

const ACTION_LABEL = {
  missing: "Install",
  stale: "Repair",
  foreign: "Replace",
  blocked: "Install",
} as const;

function CliLinkRow() {
  const transport = useTransport();
  const client = useQueryClient();
  const status = useQuery({
    queryKey: appKeys.cliLink,
    queryFn: () => unwrap(transport.cliLinkStatus()),
  });
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);

  // A failed install may still have changed the link; show its real state.
  const recheck = () =>
    void client.invalidateQueries({ queryKey: appKeys.cliLink });

  const install = async () => {
    setPending(true);
    setFailure(null);
    try {
      const result = await transport.cliInstallLink();
      if (result.status === "error") {
        console.error("[settings] cli_install_link failed", result.error);
        setFailure(
          "message" in result.error ? result.error.message : result.error.kind
        );
        recheck();
      } else {
        client.setQueryData(appKeys.cliLink, result.data);
      }
    } catch (error) {
      console.error("[settings] cli_install_link rejected", error);
      setFailure(String(error));
      recheck();
    } finally {
      setPending(false);
      setConfirming(false);
    }
  };

  const link = status.data;
  const message = failure
    ? `Couldn't install: ${failure}`
    : link
      ? linkMessage(link)
      : status.isError
        ? `Couldn't check: ${status.error.message}`
        : "Checking…";
  return (
    <SettingsRow
      label="statglass command"
      sub={<span role="status">{message}</span>}
    >
      {link && link.state !== "current" && (
        <Button
          variant="outline"
          size="sm"
          disabled={pending || link.state === "blocked" || link.target === null}
          onClick={
            link.state === "foreign" ? () => setConfirming(true) : install
          }
        >
          {ACTION_LABEL[link.state]}
        </Button>
      )}
      {link?.state === "foreign" && (
        <ConfirmDialog
          open={confirming}
          onOpenChange={setConfirming}
          className="sm:max-w-sm"
          title="Replace the statglass link?"
          description={
            <>
              {tilde(link.link_path)} points to{" "}
              {tilde(link.points_to ?? "something else")}. Replacing it makes
              statglass run this app's command-line tool. The file it points to
              isn't changed.
            </>
          }
          confirmLabel="Replace link"
          confirmVariant="destructive"
          onConfirm={install}
          busy={pending}
        />
      )}
    </SettingsRow>
  );
}

function SkillRow() {
  const [copied, setCopied] = useState<"idle" | "copied" | "failed">("idle");
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(SKILL_COMMAND);
      setCopied("copied");
    } catch (error) {
      console.error("[settings] copy failed", error);
      setCopied("failed");
    }
  };
  return (
    <SettingsRow
      label="Coding agents"
      sub={
        <span role="status">
          {copied === "failed" ? (
            `Couldn't copy. Run ${SKILL_COMMAND} in Terminal.`
          ) : (
            <>
              Run <span className="text-foreground">{SKILL_COMMAND}</span> to
              teach Claude Code and similar agents to use statglass.
            </>
          )}
        </span>
      }
    >
      <Button variant="outline" size="sm" onClick={copy}>
        {copied === "copied" ? "Copied" : "Copy command"}
      </Button>
    </SettingsRow>
  );
}

/** "Command line" (D-126): the CLI's access switch, its PATH link and the agent skill. */
export function CommandLinePanel({ enabled }: { enabled: boolean }) {
  const write = useWriteSettings();
  return (
    <SettingsPanel title="Command line">
      <SettingsRow
        label="Allow command-line access"
        sub="Tools and coding agents running as you on this Mac can read StatGlass history, including your other machines. Read-only."
      >
        <Switch
          aria-label="Allow command-line access"
          checked={enabled}
          onCheckedChange={(on) => write({ cli_access: { enabled: on } })}
        />
      </SettingsRow>
      <CliLinkRow />
      <SkillRow />
    </SettingsPanel>
  );
}
