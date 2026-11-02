import { expect, it } from "vitest";
import { createCpuSampler, sampleHistory } from "./live-sample";

it("gives every visitor the same trace, within a CPU's range", () => {
  const a = sampleHistory(createCpuSampler(), 600);
  const b = sampleHistory(createCpuSampler(), 600);
  expect(a).toEqual(b);
  expect(Math.min(...a)).toBeGreaterThanOrEqual(3);
  expect(Math.max(...a)).toBeLessThanOrEqual(95);
  // Bursts happen: a trace that never leaves idle would look dead.
  expect(Math.max(...a)).toBeGreaterThan(35);
});
