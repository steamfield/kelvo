import { usePlayback } from "@steamfield/ui/react/playback";
import { type RefObject, useEffect, useLayoutEffect, useRef } from "react";

/**
 * Calls `tick` every `intervalMs` while `target` is on screen and the tab is
 * visible. Under reduced motion it never ticks, so demos hold their first frame.
 */
export function useOnScreenInterval(
  target: RefObject<Element | null>,
  tick: () => void,
  intervalMs = 1000
) {
  const latest = useRef(tick);
  latest.current = tick;

  // The gate holds the on-screen, tab and motion conditions. Reduced motion
  // holds the first frame: it never plays, rather than jumping to an end.
  const playback = usePlayback<Element>({
    threshold: 0,
    reducedPolicy: "hold",
  });
  useLayoutEffect(() => {
    playback.ref.current = target.current;
  });

  const { playing } = playback;
  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => latest.current(), intervalMs);
    return () => clearInterval(timer);
  }, [playing, intervalMs]);
}
