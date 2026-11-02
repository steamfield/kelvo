import { usePlayback } from "@steamfield/ui/react/playback";
import { useEffect, useRef } from "react";

/**
 * Calls `tick` every `intervalMs` while the element the returned ref is
 * attached to is on screen and the tab is visible. Under reduced motion it
 * never ticks, so demos hold their first frame.
 */
export function useOnScreenInterval(tick: () => void, intervalMs = 1000) {
  const latest = useRef(tick);
  latest.current = tick;

  // The gate holds the on-screen, tab and motion conditions. Reduced motion
  // holds the first frame: it never plays, rather than jumping to an end.
  const { ref, playing } = usePlayback<Element>({
    threshold: 0,
    reducedPolicy: "hold",
  });

  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => latest.current(), intervalMs);
    return () => clearInterval(timer);
  }, [playing, intervalMs]);

  return ref;
}
