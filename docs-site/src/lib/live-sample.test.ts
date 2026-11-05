import { expect, it } from "vitest";
import { createCpuSampler, moduleSamplers, sampleHistory } from "./live-sample";

it("gives every visitor the same trace, within a CPU's range", () => {
  const a = sampleHistory(createCpuSampler(), 600);
  const b = sampleHistory(createCpuSampler(), 600);
  expect(a).toEqual(b);
  expect(Math.min(...a)).toBeGreaterThanOrEqual(3);
  expect(Math.max(...a)).toBeLessThanOrEqual(95);
  // Bursts happen: a trace that never leaves idle would look dead.
  expect(Math.max(...a)).toBeGreaterThan(35);
});

it.each(["gpu", "memory", "network"] as const)(
  "keeps simulated %s readings deterministic and within their native range",
  (module) => {
    const values = sampleHistory(moduleSamplers[module](), 600);
    expect(values).toEqual(sampleHistory(moduleSamplers[module](), 600));
    expect(Math.min(...values)).toBeGreaterThan(0);
    expect(Math.max(...values)).toBeLessThanOrEqual(
      module === "memory" ? 32 : module === "network" ? 50 : 100
    );
    expect(new Set(values).size).toBeGreaterThan(10);
  }
);
