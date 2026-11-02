import { usePlayback } from "@steamfield/ui/react/playback";
import { type RefObject, useEffect, useLayoutEffect, useRef } from "react";
import { type DemoMessage, readDemoMessage, setDemoVisible } from "@/lib/demo";

/**
 * Wires a mounted demo iframe to the page: hands its messages to `onMessage`
 * (same origin and from this frame only), and pauses it while it is off
 * screen or the tab is hidden, so an unseen demo costs no CPU. Scrolling
 * away pauses it only after `OFFSCREEN_GRACE_MS`: on resume the app shows
 * "Stale" until its next check, which a quick scroll back shouldn't see.
 *
 * When the frame is on screen and the tab is visible comes from the
 * library's playback gate; telling the iframe to show or hide is local.
 */
export const OFFSCREEN_GRACE_MS = 5000;

export function useDemoFrame(
  frame: RefObject<HTMLIFrameElement | null>,
  mounted: boolean,
  onMessage: (message: DemoMessage) => void
) {
  // The frame renders after this hook first runs, and the gate reads its
  // element when it is created. Hand it the frame once there is one: `mounted`
  // is a gate option, so flipping it builds a gate over the real element.
  const playback = usePlayback<HTMLIFrameElement>({
    autoStart: mounted,
    // Any part of the frame counts, as the page's own observer did.
    threshold: 0,
    leaveGraceMs: OFFSCREEN_GRACE_MS,
    // The demo is not an animation: reduced motion does not hide it.
    motion: "full",
  });
  useLayoutEffect(() => {
    playback.ref.current = mounted ? frame.current : null;
  });

  // The gate reports "not playing" until the observer's first report, and
  // the demo starts out running. So a frame that has never been seen is hidden
  // only after the same grace period, and one that comes into view is shown at once.
  const { ready, playing } = playback;
  const seen = useRef(false);
  useEffect(() => {
    const el = frame.current;
    if (!mounted) seen.current = false;
    if (!mounted || !ready || !el) return;
    if (playing) {
      seen.current = true;
      setDemoVisible(el, true);
      return;
    }
    if (!seen.current) {
      seen.current = true;
      const hide = setTimeout(
        () => setDemoVisible(el, false),
        OFFSCREEN_GRACE_MS
      );
      return () => clearTimeout(hide);
    }
    setDemoVisible(el, false);
    return undefined;
  }, [frame, mounted, ready, playing]);

  useEffect(() => {
    const el = frame.current;
    if (!mounted || !el) return;
    const onWindowMessage = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      if (event.source !== el.contentWindow) return;
      const message = readDemoMessage(event.data);
      if (message) onMessage(message);
    };
    window.addEventListener("message", onWindowMessage);
    return () => window.removeEventListener("message", onWindowMessage);
  }, [frame, mounted, onMessage]);
}
