import { useState } from "react";
import { createTraySampler } from "@/lib/tray-sample";
import { useOnScreenInterval } from "@/lib/use-on-screen-interval";

/**
 * Simulated menu bar readings that tick every 2 seconds, the menu bar's own
 * background cadence, while `target` is on screen and the tab is visible.
 */
export function useLiveTray() {
  const [next] = useState(() => createTraySampler());
  const [readings, setReadings] = useState(next);
  const ref = useOnScreenInterval(() => setReadings(next()), 2000);
  return { ref, readings };
}
