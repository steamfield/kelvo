import type { TrayReadings, TrayUnits } from "@statglass/widgets";
import { createCpuSampler } from "@/lib/live-sample";

/** Spark and GPU history lengths, as the menu bar's own graphs draw them. */
const CPU_HISTORY = 20;
const GPU_HISTORY = 9;
const P_CORES = 6;
const E_CORES = 4;

const round = (v: number, places = 0) => {
  const f = 10 ** places;
  return Math.round(v * f) / f;
};

/**
 * Simulated menu bar readings for the landing page: seeded CPU, GPU and
 * per-core walks, with temperature and power following the CPU, so every
 * visitor sees the same plausible menu bar. Each call to `next` is one tick.
 */
export function createTraySampler(seed = 7) {
  const cpu = createCpuSampler(seed);
  const gpu = createCpuSampler(seed + 4);
  const cores = Array.from({ length: P_CORES + E_CORES }, (_, i) =>
    createCpuSampler(seed + 10 + i)
  );
  let cpuHistory: number[] = [];
  let gpuHistory: number[] = [];
  let tick = 0;

  const step = (): TrayReadings => {
    tick += 1;
    const c = cpu();
    // The GPU idles lower than the CPU.
    const g = round(Math.max(1, gpu() * 0.6), 1);
    cpuHistory = [...cpuHistory, c].slice(-CPU_HISTORY);
    gpuHistory = [...gpuHistory, g].slice(-GPU_HISTORY);
    const load = cores.map((next) => next());
    return {
      cpu: c,
      gpu: g,
      mem: round(58 + 4 * Math.sin(tick / 40), 1),
      temp: round(44 + c * 0.35),
      power: round(3.5 + c * 0.32 + g * 0.12, 1),
      netUp: round(18_000 + c * 900),
      netDown: round(240_000 + c * 21_000),
      diskRate: round(120_000 + c * 8_000),
      diskUsed: 61,
      battery: 82,
      cpuHistory,
      gpuHistory,
      cores: [load.slice(0, P_CORES), load.slice(P_CORES)],
    };
  };

  // Start with full graphs rather than a line growing from the right edge.
  for (let i = 1; i < CPU_HISTORY; i += 1) step();
  return step;
}

/** The app's default units: °C and bytes per second. */
export const UNITS: TrayUnits = {
  temperature: "celsius",
  network: "bytes_per_sec",
};
