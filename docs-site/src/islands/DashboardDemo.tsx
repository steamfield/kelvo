import { useCallback, useEffect, useRef, useState } from "react";
import { track } from "@/lib/analytics";
import {
  DASHBOARD_HEIGHT,
  DASHBOARD_MIN_WIDTH,
  type DemoMessage,
  dashboardDemoUrl,
} from "@/lib/demo";
import { EVENTS } from "@/lib/events";
import { useDemoFrame } from "@/lib/use-demo-frame";
import { useFrameReady } from "@/lib/use-frame-ready";

/** The section's content box must fit the dashboard's minimum width. */
const FITS_QUERY = `(min-width: ${DASHBOARD_MIN_WIDTH + 64}px)`;

/**
 * The real dashboard at native size, loaded as the reader scrolls near it,
 * on screens that fit its minimum width. Narrower screens, and the page
 * before scripts run, get a link that opens it in its own tab instead of a
 * scaled-down copy nobody can read.
 */
export function DashboardDemo({ route }: { route: string }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const slot = useRef<HTMLDivElement>(null);
  const [fits, setFits] = useState(false);
  const [near, setNear] = useState(false);

  useEffect(() => {
    const query = matchMedia(FITS_QUERY);
    const sync = () => setFits(query.matches);
    sync();
    query.addEventListener("change", sync);
    return () => query.removeEventListener("change", sync);
  }, []);

  useEffect(() => {
    const el = slot.current;
    if (!fits || near || !el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) setNear(true);
      },
      { rootMargin: "400px 0px" }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [fits, near]);

  const mounted = fits && near;
  // The dashboard has nothing to report up; sizes and links stay inside it.
  const onMessage = useCallback((_: DemoMessage) => {}, []);
  const frameRef = useDemoFrame(frame, mounted, onMessage);
  const ready = useFrameReady(frame, mounted);

  const href = dashboardDemoUrl(route);
  return (
    <div ref={slot} className="flex flex-col gap-3">
      {fits && (
        <div
          className="w-full overflow-hidden rounded-card border border-border bg-background"
          style={{ height: DASHBOARD_HEIGHT }}
        >
          {mounted && (
            <iframe
              ref={frameRef}
              src={href}
              title="StatGlass dashboard, live demo"
              className={`block h-full w-full border-0 transition-opacity duration-(--motion-fast) ${ready ? "opacity-100" : "opacity-0"}`}
            />
          )}
        </div>
      )}
      <p className="text-sm text-muted-foreground">
        <a
          href={href}
          target="_blank"
          rel="noopener"
          onClick={() => track(EVENTS.demoStarted, { from: "dashboard_link" })}
          className="text-link underline underline-offset-2"
        >
          Open the dashboard demo in its own tab
        </a>
        {fits ? "" : ". It needs a window at least 1024 px wide."}
      </p>
    </div>
  );
}
