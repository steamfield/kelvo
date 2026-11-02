import { RefreshCw, TriangleAlert } from "lucide-react";
import { useEffect } from "react";
import { Link, useLocation, useNavigate, useRouteError } from "react-router";
import { Button } from "~/components/ui/button";
import { useHostSelection } from "~/lib/host-selection";
import { useTransport } from "~/lib/transport-context";
import { cn } from "~/lib/utils";

/** Unexpected render failures stay local to the view, with a way back out. */
export function RouteError({ embedded = false }: { embedded?: boolean }) {
  const error = useRouteError();
  const location = useLocation();
  const navigate = useNavigate();
  const transport = useTransport();
  const selection = useHostSelection();
  const host = selection?.hosts.find((h) => h.id === selection.selected);
  const hostId = host?.id;
  const windowLabel = transport.windowLabel();
  const message = error instanceof Error ? error.message : String(error);
  useEffect(() => {
    console.error("[route] render error", {
      path: location.pathname,
      hostId,
      window: windowLabel,
      error,
    });
  }, [error, location.pathname, hostId, windowLabel]);
  return (
    <section
      aria-labelledby="view-error-title"
      className={cn(
        "flex flex-col items-start gap-4 bg-background text-foreground",
        embedded ? "py-10" : "min-h-svh justify-center p-8"
      )}
    >
      <TriangleAlert aria-hidden className="size-6 text-muted-foreground" />
      <div className="max-w-md space-y-2">
        <h1 id="view-error-title" className="font-[590] text-[18px]">
          This view hit a snag
        </h1>
        <p className="text-[13px] text-muted-foreground">
          {host
            ? `StatGlass couldn’t display this view for ${host.display_name}.`
            : "StatGlass couldn’t display this view."}{" "}
          Try again, or open Hosts to choose another machine.
        </p>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          onClick={() =>
            void navigate(location.pathname + location.search, {
              replace: true,
            })
          }
        >
          <RefreshCw aria-hidden className="size-3.5" />
          Try again
        </Button>
        <Button size="sm" variant="outline" asChild>
          <Link to="/dashboard/hosts">Open Hosts</Link>
        </Button>
      </div>
      <details className="w-full max-w-lg text-[12px] text-muted-foreground">
        <summary className="w-fit cursor-pointer hover:text-foreground">
          Technical details
        </summary>
        <div className="wrap-anywhere mt-2 space-y-1 rounded-md border border-border-subtle bg-muted/30 p-3">
          <p>{message}</p>
          <p>View: {location.pathname}</p>
          {hostId && <p>Host: {hostId}</p>}
          <p>Window: {windowLabel}</p>
        </div>
      </details>
    </section>
  );
}
