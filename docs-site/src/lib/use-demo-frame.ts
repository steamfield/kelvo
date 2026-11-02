import { type RefObject, useEffect } from "react";
import { type DemoMessage, readDemoMessage, setDemoVisible } from "@/lib/demo";

/**
 * Wires a mounted demo iframe to the page: hands its messages to `onMessage`
 * (same origin and from this frame only), and pauses it while it is off
 * screen or the tab is hidden, so an unseen demo costs no CPU. Scrolling
 * away pauses it only after `OFFSCREEN_GRACE_MS`: on resume the app shows
 * "Stale" until its next check, which a quick scroll back shouldn't see.
 */
export const OFFSCREEN_GRACE_MS = 5000;

export function useDemoFrame(
  frame: RefObject<HTMLIFrameElement | null>,
  mounted: boolean,
  onMessage: (message: DemoMessage) => void
) {
  useEffect(() => {
    const el = frame.current;
    if (!mounted || !el) return;
    const onWindowMessage = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      if (event.source !== el.contentWindow) return;
      const message = readDemoMessage(event.data);
      if (message) onMessage(message);
    };
    let onScreen = true;
    let grace: ReturnType<typeof setTimeout> | undefined;
    const sync = () =>
      setDemoVisible(el, onScreen && document.visibilityState === "visible");
    const observer = new IntersectionObserver(([entry]) => {
      clearTimeout(grace);
      onScreen = entry?.isIntersecting ?? true;
      if (onScreen) sync();
      else grace = setTimeout(sync, OFFSCREEN_GRACE_MS);
    });
    observer.observe(el);
    window.addEventListener("message", onWindowMessage);
    document.addEventListener("visibilitychange", sync);
    return () => {
      clearTimeout(grace);
      observer.disconnect();
      window.removeEventListener("message", onWindowMessage);
      document.removeEventListener("visibilitychange", sync);
    };
  }, [frame, mounted, onMessage]);
}
