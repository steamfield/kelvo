import {
  type Accent,
  InlineBar,
  ModuleCard,
  StreamArea,
} from "@statglass/widgets";
import type { ModuleKey } from "@/lib/live-sample";
import { useLiveSeries } from "@/lib/use-live-series";
export type { ModuleKey } from "@/lib/live-sample";

export type ModuleLiveCardProps = { module: ModuleKey; caption?: string };

const configs = {
  cpu: { accent: "cpu", title: "CPU", max: 100, unit: "%" },
  gpu: { accent: "gpu", title: "GPU", max: 100, unit: "%" },
  memory: { accent: "mem", title: "Memory", max: 32, unit: " GiB" },
  network: { accent: "net", title: "Network", max: 50, unit: " MB/s" },
} satisfies Record<
  ModuleKey,
  { accent: Accent; title: string; max: number; unit: string }
>;

export function ModuleReading({ module }: { module: ModuleKey }) {
  const { ref, values, tEndMs } = useLiveSeries(60, 1000, module);
  const total = values.at(-1) ?? 0;
  const config = configs[module];
  const value = `${module === "cpu" ? Math.round(total) : total.toFixed(1)}${config.unit}`;
  const rows =
    module === "cpu"
      ? [
          {
            label: "User",
            value: `${(total * 0.68).toFixed(1)}%`,
            fraction: (total * 0.68) / 100,
          },
          {
            label: "System",
            value: `${(total * 0.32).toFixed(1)}%`,
            fraction: (total * 0.32) / 100,
          },
        ]
      : module === "gpu"
        ? [
            { label: "Utilization", value, fraction: total / 100 },
            {
              label: "Frequency",
              value: `${(0.4 + total / 100).toFixed(2)} GHz`,
              fraction: (0.4 + total / 100) / 1.4,
            },
          ]
        : module === "memory"
          ? [
              {
                label: "App",
                value: `${(total * 0.65).toFixed(1)} GiB`,
                fraction: (total * 0.65) / 32,
              },
              {
                label: "Wired",
                value: `${(total * 0.25).toFixed(1)} GiB`,
                fraction: (total * 0.25) / 32,
              },
              {
                label: "Compressed",
                value: `${(total * 0.1).toFixed(1)} GiB`,
                fraction: (total * 0.1) / 32,
              },
            ]
          : [
              { label: "Download", value, fraction: total / 50 },
              {
                label: "Upload",
                value: `${(total * 0.18).toFixed(1)} MB/s`,
                fraction: (total * 0.18) / 50,
              },
            ];
  return (
    <div ref={ref} className="w-full max-w-[360px]">
      <ModuleCard accent={config.accent} title={config.title} value={value}>
        <StreamArea
          series={[{ key: module, values, step: 1 }]}
          tEndMs={tEndMs}
          intervalMs={1000}
          yMax={config.max}
          accent={config.accent}
          height={52}
          ariaLabel={`${config.title}, simulated reading, last minute, ${value}`}
          ceilingLabel={`${config.max}${config.unit}`}
          windowLabel="60s"
          gridlines={[config.max / 2]}
        />
        <div className="flex flex-col gap-[5px] pt-1.5">
          {rows.map((row, index) => (
            <InlineBar
              key={row.label}
              {...row}
              rampStep={index === 0 ? 1 : index === 1 ? 2 : 3}
              layout="row"
            />
          ))}
        </div>
      </ModuleCard>
    </div>
  );
}

export function ModuleLiveCard({ module, caption }: ModuleLiveCardProps) {
  return (
    <figure
      data-demo=""
      data-nosnippet=""
      className="mx-auto w-full max-w-[360px] text-base leading-normal font-[510]"
    >
      <ModuleReading module={module} />
      <figcaption className="mt-3 text-xs text-muted-foreground">
        Simulated reading{caption ? `. ${caption}` : ""}
      </figcaption>
    </figure>
  );
}
