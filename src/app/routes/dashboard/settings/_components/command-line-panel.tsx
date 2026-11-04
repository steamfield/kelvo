import type { CliLinkStatus } from "@core/generated/bindings";
import { appKeys } from "@core/query-keys";
import { unwrap } from "@core/transport";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { SettingsPanel, SettingsRow } from "~/components/settings-row";
import { Button } from "~/components/ui/button";
import { Switch } from "~/components/ui/switch";
import { useWriteSettings } from "~/hooks/use-write-settings";
import { useTransport } from "~/lib/transport-context";

const SKILL_COMMAND = "statglass skill install";

/** `/Users/name/.local/bin/statglass` as a shell user writes it. */
const tilde = (path: string) => path.replace(/^\/Users\/[^/]+\//, "~/");

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
      return `${path} points to an older copy of StatGlass.`;
    case "blocked":
      return `${path} is a file, not a link. Move it to install.`;
  }
}

function CliLinkRow() {
  const transport = useTransport();
  const client = useQueryClient();
  const status = useQuery({
    queryKey: appKeys.cliLink,
    queryFn: () => unwrap(transport.cliLinkStatus()),
  });
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

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
      } else {
        client.setQueryData(appKeys.cliLink, result.data);
      }
    } catch (error) {
      console.error("[settings] cli_install_link rejected", error);
      setFailure(String(error));
    } finally {
      setPending(false);
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
          onClick={install}
        >
          {link.state === "stale" ? "Repair" : "Install"}
        </Button>
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
