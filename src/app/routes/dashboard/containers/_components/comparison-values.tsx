import {
  type ComparisonMetric,
  pointCovered,
  pointValue,
} from "@core/chart-math/series-lines";
import type {
  DockerContainer,
  DockerHistoryPoint,
} from "@core/generated/bindings";
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/components/ui/table";
import type { ComparisonSeries } from "./comparison-chart";
export function ComparisonValues({
  series,
  times,
  metric,
  displayName,
  clock,
  statistics,
  format,
}: {
  series: readonly ComparisonSeries[];
  times: readonly number[];
  metric: ComparisonMetric;
  displayName: (container: DockerContainer) => string;
  clock: (time: number) => string;
  statistics: (point: DockerHistoryPoint) => string;
  format: (value: number | null) => string;
}) {
  const pointsBySeries = new Map(
    series.map((s) => [
      s.container.id,
      new Map(s.points.map((p) => [p.time_ms, p])),
    ])
  );
  return (
    <div className="h-60 overflow-auto">
      <Table
        className="figures w-full caption-top text-[11px]"
        aria-label="Container history values"
      >
        <TableCaption className="mt-0 text-left text-[11px] text-muted-foreground">
          Per bucket: min / avg / max
        </TableCaption>
        <TableHeader variant="plain" className="sticky top-0 bg-card">
          <TableRow variant="plain">
            <TableHead
              variant="plain"
              className="p-2 text-left font-bold text-inherit"
            >
              Time
            </TableHead>
            {series.map((s) => (
              <TableHead
                variant="plain"
                key={s.container.id}
                className="p-2 text-right font-bold text-inherit"
              >
                {displayName(s.container)}
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {[...times].reverse().map((time) => (
            <TableRow variant="plain" key={time}>
              <TableCell className="whitespace-nowrap p-2">
                {clock(time)}
              </TableCell>
              {series.map((s) => {
                const point = pointsBySeries.get(s.container.id)?.get(time);
                const value = point ? pointValue(point, metric) : null;
                return (
                  <TableCell
                    key={s.container.id}
                    className="whitespace-nowrap p-2 text-right"
                  >
                    {point && value !== null ? statistics(point) : format(null)}
                    {value !== null && point && !pointCovered(point, metric)
                      ? " partial"
                      : ""}
                  </TableCell>
                );
              })}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
