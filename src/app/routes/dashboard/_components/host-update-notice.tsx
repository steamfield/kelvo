import { hostKeys } from "@core/query-keys";
import {
  hostUpdateNotice,
  type HostUpdateNotice as Notice,
} from "@core/remote-hosts";
import { unwrap } from "@core/transport";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Info, X } from "lucide-react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { useLocalUpdateStatus } from "~/hooks/use-local-update-status";
import { useRemoteHosts } from "~/hooks/use-remote-hosts";
import { useTransport } from "~/lib/transport-context";

const hosts = (n: number) => (n === 1 ? "1 host" : `${n} hosts`);

function noticeText(notice: Notice): {
  text: string;
  action: string;
  href: string;
} {
  switch (notice.kind) {
    case "required":
      return {
        text: `${hosts(notice.count)} can’t connect until ${notice.count === 1 ? "it’s" : "they’re"} updated.`,
        action: "Review hosts",
        href: "/dashboard/hosts",
      };
    case "hosts":
      return {
        text: `${hosts(notice.count)} ${notice.count === 1 ? "is" : "are"} running an older Kelvo.`,
        action: "Review hosts",
        href: "/dashboard/hosts",
      };
    case "local_first":
      return {
        text: `Kelvo ${notice.version} is available. Update this Mac first, then your hosts.`,
        action: "Open Settings",
        href: "/dashboard/settings",
      };
    case "local_restart":
      return {
        text: `Kelvo ${notice.version} is installing on this Mac. Update your hosts after it restarts.`,
        action: "Open Settings",
        href: "/dashboard/settings",
      };
  }
}

/**
 * One sentence, its cyan action (design-system.md's text-link rule, as in
 * `UnsupportedNotice`) and a dismiss control. A host that can't connect has no
 * dismiss: the notice stays until it is updated.
 */
export function HostUpdateNoticeView({
  notice,
  onOpen,
  onDismiss,
}: {
  notice: Notice;
  onOpen: (href: string) => void;
  onDismiss: (key: string) => void;
}) {
  const { text, action, href } = noticeText(notice);
  return (
    <section
      aria-label="Host updates"
      className="mb-4 flex items-start gap-2 border-border-subtle border-b pb-3"
    >
      <Info
        aria-hidden
        className="mt-px size-3.5 shrink-0 text-muted-foreground"
        strokeWidth={1.5}
      />
      <p className="m-0 flex-1 font-normal text-[12px] text-muted-foreground leading-normal">
        {text}{" "}
        <button
          type="button"
          onClick={() => onOpen(href)}
          className="rounded-chip text-link outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
        >
          {action}
        </button>
      </p>
      {notice.kind !== "required" && (
        <button
          type="button"
          aria-label="Dismiss"
          onClick={() => onDismiss(notice.key)}
          className="rounded-chip text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
        >
          <X aria-hidden className="size-3.5" strokeWidth={1.5} />
        </button>
      )}
    </section>
  );
}

/** Hosts behind this Mac's version, on every dashboard page but Hosts and Settings (D-123). */
export function HostUpdateNotice() {
  const transport = useTransport();
  const cache = useQueryClient();
  const navigate = useNavigate();
  const remotes = useRemoteHosts();
  const [local] = useLocalUpdateStatus();
  const dismissed = useQuery({
    queryKey: hostKeys.updateNotice(),
    queryFn: () => unwrap(transport.hostUpdateNoticeDismissed()),
  });
  if (!remotes.data || dismissed.isPending) return null;
  const notice = hostUpdateNotice(remotes.data, local, dismissed.data ?? null);
  if (!notice) return null;
  const dismiss = async (key: string) => {
    try {
      await unwrap(transport.dismissHostUpdateNotice(key));
      cache.setQueryData(hostKeys.updateNotice(), key);
    } catch (error) {
      console.error("[dashboard] dismissing the host update notice failed", {
        key,
        error,
      });
      toast.error("Couldn’t dismiss the notice. Try again.");
    }
  };
  return (
    <HostUpdateNoticeView
      notice={notice}
      onOpen={(href) => void navigate(href)}
      onDismiss={(key) => void dismiss(key)}
    />
  );
}
