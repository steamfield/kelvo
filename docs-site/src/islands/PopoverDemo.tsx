import { useCallback, useEffect, useRef, useState } from "react";
import { CpuStreamDemo } from "@/islands/CpuStreamDemo";
import { track } from "@/lib/analytics";
import {
  type DemoMessage,
  dashboardDemoUrl,
  POPOVER_COLLAPSED_HEIGHT,
  POPOVER_MAX_HEIGHT,
  popoverDemoUrl,
} from "@/lib/demo";
import { EVENTS } from "@/lib/events";
import { useDemoFrame } from "@/lib/use-demo-frame";
import { useFrameReady } from "@/lib/use-frame-ready";

/** Wide enough to load the demo without asking (Tailwind's `sm`). */
const AUTOLOAD_QUERY = "(min-width: 640px)";

/**
 * The hero's live demo: the real popover at its native 360 px width, opened
 * with every card collapsed. The frame takes that collapsed height and keeps
 * it: expanding a card scrolls inside the popover instead of growing the
 * hero. Phones get the CPU card on simulated data and a button, because the
 * app is ~460 KB to download.
 */
export function PopoverDemo() {
  const frame = useRef<HTMLIFrameElement>(null);
  const [wanted, setWanted] = useState(false);
  const [height, setHeight] = useState(POPOVER_COLLAPSED_HEIGHT);
  const sized = useRef(false);

  useEffect(() => {
    if (matchMedia(AUTOLOAD_QUERY).matches) setWanted(true);
  }, []);

  const onMessage = useCallback((message: DemoMessage) => {
    if (message.type === "size") {
      if (sized.current) return;
      sized.current = true;
      setHeight(Math.min(Math.round(message.height), POPOVER_MAX_HEIGHT));
    } else {
      track(EVENTS.demoDashboardOpened, { route: message.route });
      window.open(dashboardDemoUrl(message.route), "_blank", "noopener");
    }
  }, []);
  const frameRef = useDemoFrame(frame, wanted, onMessage);
  const ready = useFrameReady(frame, wanted);

  return (
    <figure className="flex w-full flex-col items-center gap-3">
      <div
        className={`${wanted ? "block" : "hidden sm:block"} w-full max-w-[360px] overflow-hidden rounded-panel border border-border bg-background`}
        style={{ height }}
      >
        {wanted && (
          <iframe
            ref={frameRef}
            src={popoverDemoUrl()}
            title="StatGlass popover, live demo"
            className={`ph-no-capture block h-full w-full border-0 transition-opacity duration-(--motion-fast) ${ready ? "opacity-100" : "opacity-0"}`}
          />
        )}
      </div>
      {!wanted && (
        <div className="flex w-full flex-col items-center gap-3 sm:hidden">
          <CpuStreamDemo />
          <button
            type="button"
            onClick={() => {
              track(EVENTS.demoStarted, { from: "hero" });
              setWanted(true);
            }}
            className="inline-flex min-h-11 items-center rounded-control border border-border bg-card px-4 text-sm font-[510] text-foreground hover:bg-secondary"
          >
            Load the live popover
          </button>
        </div>
      )}
      <figcaption className="max-w-[360px] text-center text-xs text-muted-foreground">
        <span className={wanted ? "" : "hidden sm:inline"}>
          The real popover, running on simulated data. Click a card to open its
          dashboard page.
        </span>
        {!wanted && (
          <span className="sm:hidden">
            The popover's CPU card, drawn by the app's own components on
            simulated data.
          </span>
        )}
      </figcaption>
    </figure>
  );
}
