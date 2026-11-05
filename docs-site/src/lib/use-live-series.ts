import { useState } from "react";
import {
  type ModuleKey,
  moduleSamplers,
  sampleHistory,
} from "@/lib/live-sample";
import { useOnScreenInterval } from "@/lib/use-on-screen-interval";

/**
 * A fixed-length series that gains a sample every `intervalMs` while `target`
 * is on screen and the tab is visible. Under reduced motion it stays on its
 * first frame. `tEndMs` is the last sample's time, as StreamArea expects.
 */
export function useLiveSeries(
  length: number,
  intervalMs = 1000,
  module: ModuleKey = "cpu"
) {
  const [state, setState] = useState(() => {
    const next = moduleSamplers[module]();
    return { next, values: sampleHistory(next, length), tEndMs: 0 };
  });
  const ref = useOnScreenInterval(
    () =>
      setState((s) => ({
        ...s,
        values: [...s.values.slice(1), s.next()],
        tEndMs: Date.now(),
      })),
    intervalMs
  );
  return { ref, values: state.values, tEndMs: state.tEndMs };
}
