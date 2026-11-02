import { expect, it } from "vitest";
import { createTraySampler } from "@/lib/tray-sample";

it("starts with full menu bar graphs and keeps them at their drawn length", () => {
  const next = createTraySampler();
  const first = next();
  expect(first.cpuHistory).toHaveLength(20);
  expect(first.gpuHistory).toHaveLength(9);
  for (let i = 0; i < 50; i += 1) next();
  const later = next();
  expect(later.cpuHistory).toHaveLength(20);
  expect(later.cpuHistory?.at(-1)).toBe(later.cpu);
  expect(later.cores?.map((c) => c.length)).toEqual([6, 4]);
});

it("keeps every percentage in range and is the same for every visitor", () => {
  const a = createTraySampler();
  const b = createTraySampler();
  for (let i = 0; i < 200; i += 1) {
    const r = a();
    expect(b()).toEqual(r);
    for (const v of [r.cpu, r.gpu, r.mem, ...(r.cores ?? []).flat()]) {
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(100);
    }
  }
});
