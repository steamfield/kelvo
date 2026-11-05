/**
 * Simulated CPU load for the landing page's live widget: a seeded
 * (Park–Miller) random walk around a light load with occasional bursts, so
 * every visitor sees the same plausible trace.
 */
export function createCpuSampler(seed = 7) {
  let state = seed % 2147483647 || 1;
  const random = () => {
    state = (state * 16807) % 2147483647;
    return (state - 1) / 2147483646;
  };
  let level = 14;
  let burst = 0;
  return function next(): number {
    if (burst === 0 && random() < 0.04) burst = 3 + Math.floor(random() * 5);
    const target = burst > 0 ? 48 : 14;
    if (burst > 0) burst -= 1;
    level += (target - level) * 0.35 + (random() - 0.5) * 6;
    level = Math.min(95, Math.max(3, level));
    return Math.round(level * 10) / 10;
  };
}

/** The first `length` samples of a sampler, oldest first. */
export function sampleHistory(next: () => number, length: number): number[] {
  return Array.from({ length }, () => next());
}

/** A seeded walk in each module's native unit: percent, GiB or MB/s. */
export type ModuleKey = "cpu" | "gpu" | "memory" | "network";

function rangedSampler(base: number, peak: number, max: number, seed: number) {
  const cpu = createCpuSampler(seed);
  return () => Math.round(Math.min(max, base + (cpu() / 100) * peak) * 10) / 10;
}

export const createGpuSampler = () => rangedSampler(8, 85, 100, 13);
export const createMemorySampler = () => rangedSampler(12, 10, 30, 19);
export const createNetworkSampler = () => rangedSampler(0.2, 45, 80, 23);

export const moduleSamplers = {
  cpu: createCpuSampler,
  gpu: createGpuSampler,
  memory: createMemorySampler,
  network: createNetworkSampler,
};
