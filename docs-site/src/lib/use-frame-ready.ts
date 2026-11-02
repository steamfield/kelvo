import { type RefObject, useEffect, useState } from "react";
import { demoPainted } from "@/lib/demo";

/** Shown anyway after this long, so a slow or changed app never stays hidden. */
const READY_TIMEOUT_MS = 4000;

/**
 * True once the demo iframe has painted in the page's theme. Until then the
 * caller keeps the frame transparent over the site's background, so loading
 * never flashes a white canvas.
 */
export function useFrameReady(
  frame: RefObject<HTMLIFrameElement | null>,
  mounted: boolean
): boolean {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const el = frame.current;
    if (!mounted || !el) return;
    let raf = 0;
    const deadline = performance.now() + READY_TIMEOUT_MS;
    const check = () => {
      const doc = el.contentDocument;
      const dark = document.documentElement.classList.contains("dark");
      if ((doc && demoPainted(doc, dark)) || performance.now() > deadline) {
        setReady(true);
        return;
      }
      raf = requestAnimationFrame(check);
    };
    raf = requestAnimationFrame(check);
    return () => cancelAnimationFrame(raf);
  }, [frame, mounted]);
  return ready;
}
