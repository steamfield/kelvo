import { InlineBar, ModuleCard, StreamArea } from "@statglass/widgets";
import { useLiveSeries } from "@/lib/use-live-series";

const SAMPLES = 60;

/**
 * The popover's CPU card, drawn with the app's own widgets on simulated
 * data. Phones see this instead of the full popover demo, which is too
 * heavy to load unasked.
 */
export function CpuStreamDemo() {
  const { ref, values, tEndMs } = useLiveSeries(SAMPLES);
  const total = values.at(-1) ?? 0;
  // A rough user/system split, enough for the bars to move with the total.
  const user = Math.round(total * 0.68 * 10) / 10;
  const system = Math.round((total - user) * 10) / 10;
  return (
    <div ref={ref} className="w-full max-w-[360px]">
      <ModuleCard accent="cpu" title="CPU" value={`${Math.round(total)}%`}>
        <StreamArea
          series={[{ key: "cpu.total", values, step: 1 }]}
          tEndMs={tEndMs}
          intervalMs={1000}
          yMax={100}
          accent="cpu"
          height={52}
          ariaLabel={`CPU, last minute, ${Math.round(total)}%`}
          ceilingLabel="100%"
          windowLabel="60s"
          gridlines={[50]}
        />
        <div className="flex flex-col gap-[5px] pt-1.5">
          <InlineBar
            label="User"
            value={`${user.toFixed(1)}%`}
            fraction={user / 100}
            layout="row"
          />
          <InlineBar
            label="System"
            value={`${system.toFixed(1)}%`}
            fraction={system / 100}
            rampStep={2}
            layout="row"
          />
        </div>
      </ModuleCard>
    </div>
  );
}
