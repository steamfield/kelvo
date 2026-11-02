import { type RefObject, useEffect, useRef } from "react";

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

  useEffect(() => {
    const el = target.current;
    if (!el) return;
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    let onScreen = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    const sync = () => {
      const run = onScreen && document.visibilityState === "visible";
      if (run && !timer)
        timer = setInterval(() => latest.current(), intervalMs);
      if (!run && timer) {
        clearInterval(timer);
        timer = undefined;
      }
    };
    const observer = new IntersectionObserver(([entry]) => {
      onScreen = entry?.isIntersecting ?? false;
      sync();
    });
    observer.observe(el);
    document.addEventListener("visibilitychange", sync);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", sync);
      if (timer) clearInterval(timer);
    };
  }, [target, intervalMs]);
}
