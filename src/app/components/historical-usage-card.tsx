import type { TimeRange } from "@core/brush";
import { recordedDate } from "@core/chart-range";
import { formatSpan } from "@core/format";
import type {
  HistoricalUsageApp,
  HistoricalUsageFigures,
  HistoricalUsageMetric,
  HistoricalUsageReply,
} from "@core/generated/bindings";
import { USAGE_HISTORY_MAX_RANGE_MS } from "@core/generated/bindings";
import {
  HISTORICAL_COLUMNS,
  HISTORICAL_LABELS,
  historicalCoverage,
  historicalValue,
} from "@core/historical-usage";
import { historyKeys } from "@core/query-keys";
import { remoteQueryText } from "@core/remote-errors";
import { useQueryClient } from "@tanstack/react-query";
import { ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  APP_CELL,
  AppsNote,
  AppsShell,
  UnrecordedRange,
} from "~/components/app-table";
import {
  type ColumnSort,
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useColumnSort,
  useDataTable,
} from "~/components/data-table";
import { HistoricalUsageCells } from "~/components/historical-usage-cells";
import { HistoricalUsageProcesses } from "~/components/historical-usage-processes";
import { Button } from "~/components/ui/button";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";
import { useHistoricalUsage } from "~/hooks/use-historical-usage";
import { useUnits } from "~/hooks/use-units";
import { cn } from "~/lib/utils";
import { useHostId } from "~/stores/host-store";
import { InitialChip } from "~/widgets/initial-chip";
import type { Accent } from "~/widgets/lib/accent";

export type HistoricalUsageSort = ColumnSort<HistoricalUsageMetric>;

export function HistoricalUsageCard({
  range,
  by,
  accent = "cpu",
  columns = HISTORICAL_COLUMNS,
  title = "Apps in this range",
  compare = false,
  sort: controlledSort,
  onSortChange,
}: {
  range: TimeRange | null;
  by: HistoricalUsageMetric;
  accent?: Accent;
  columns?: readonly HistoricalUsageMetric[];
  title?: string;
  compare?: boolean;
  sort?: HistoricalUsageSort;
  onSortChange?: (sort: HistoricalUsageSort) => void;
}) {
  const [localSort, setLocalSort] = useState<HistoricalUsageSort>({
    by,
    dir: "desc",
  });
  useEffect(() => setLocalSort({ by, dir: "desc" }), [by]);
  const selected = controlledSort ?? localSort;
  const sort = visibleSort(selected, by, columns);
  const query = useHistoricalUsage(range, sort.by);
  const names = useMemo(
    () => query.data?.apps.map((app) => app.name) ?? [],
    [query.data]
  );
  const previousRange = comparisonRange(range, compare, names.length);
  const previous = useHistoricalUsage(previousRange, sort.by, {
    appNames: names,
  });
  const host = useHostId();
  const client = useQueryClient();
  const change = (next: HistoricalUsageSort) => {
    setLocalSort(next);
    onSortChange?.(next);
  };
  if (range && range.toMs - range.fromMs > USAGE_HISTORY_MAX_RANGE_MS)
    return (
      <AppsShell title={title} accent={accent}>
        <AppsNote>
          Recorded app attribution supports intervals up to 30 days. Choose a
          shorter range to inspect apps.
        </AppsNote>
      </AppsShell>
    );
  return (
    <AppsShell
      title={title}
      accent={accent}
      aside={
        <Button
          variant="ghost"
          size="sm"
          disabled={query.isFetching || range === null}
          onClick={() => {
            if (range)
              void client.resetQueries({
                queryKey: historyKeys.usageProcessRange(
                  host,
                  range.fromMs,
                  range.toMs
                ),
              });
            void query.refetch();
            if (compare) void previous.refetch();
          }}
        >
          Refresh
        </Button>
      }
    >
      {range && (
        <AppsNote>
          <span className="figures">
            {recordedDate(range.fromMs)} to {recordedDate(range.toMs)}
          </span>{" "}
          · {Intl.DateTimeFormat().resolvedOptions().timeZone}
        </AppsNote>
      )}
      {query.isError ? (
        <AppsNote>
          <span role="alert">
            {remoteQueryText(query.error, "recorded app usage") ??
              "Recorded app usage could not be read."}
          </span>{" "}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry
          </Button>
        </AppsNote>
      ) : !query.data || !range ? (
        <AppsNote>
          <span role="status">Loading recorded apps…</span>
        </AppsNote>
      ) : (
        <>
          {query.isFetching && (
            <AppsNote>
              <span role="status">Refreshing recorded apps…</span>
            </AppsNote>
          )}
          <HistoricalCoverage data={query.data} by={sort.by} />
          {compare && previousRange && (
            <AppsNote>
              <span className="figures">
                Previous: {recordedDate(previousRange.fromMs)} to{" "}
                {recordedDate(previousRange.toMs)}
              </span>
              {previous.isFetching && " · Loading previous interval…"}
              {previous.isError && (
                <>
                  <span role="alert"> · Previous interval unavailable.</span>{" "}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void previous.refetch()}
                  >
                    Retry previous interval
                  </Button>
                </>
              )}
            </AppsNote>
          )}
          {compare && previous.data && (
            <HistoricalCoverage data={previous.data} by={sort.by} previous />
          )}
          <HistoricalUsageTable
            key={`${host}:${range.fromMs}:${range.toMs}:${sort.by}`}
            data={query.data}
            previous={previous.data}
            range={range}
            columns={columns}
            sort={sort}
            onSort={change}
            compare={compare}
          />
        </>
      )}
    </AppsShell>
  );
}

function HistoricalCoverage({
  data,
  by,
  previous = false,
}: {
  data: HistoricalUsageReply;
  by: HistoricalUsageMetric;
  previous?: boolean;
}) {
  const coverage = data.range.coverage.find((c) => c.metric === by)?.coverage;
  const resolutions = [
    ...new Set(data.range.spans.map((span) => span.resolution_ms)),
  ]
    .map(formatSpan)
    .join(", ");
  const incomplete = coverage?.causes
    .map((cause) => COVERAGE_CAUSES[cause])
    .filter((cause, index, all) => all.indexOf(cause) === index)
    .join("; ");
  return (
    <AppsNote>
      {previous ? "Previous " : ""}
      {HISTORICAL_LABELS[by]}:{" "}
      {historicalCoverage(
        coverage,
        data.range.requested_to_ms - data.range.requested_from_ms
      )}
      .{resolutions && ` Resolution: ${resolutions}.`}
      {!data.ranking_complete &&
        ` Observed ranking${incomplete ? `: ${incomplete}` : "; some use could not be recorded"}.`}
      {data.network_uncertainty.length > 0 &&
        " Some network bytes have uncertain transfer times and are excluded from exact totals."}
    </AppsNote>
  );
}

const COVERAGE_CAUSES = {
  not_recorded: "attribution was not recorded",
  pruned: "older history was removed",
  sampling_gap: "recording has gaps",
  read_failed: "a source read failed",
  no_baseline: "some first observations lacked a rate baseline",
  unreadable_process: "some processes could not be read",
  unresolved_identity: "some app identities were unknown",
  identity_limit: "some app identities could not be retained",
  uncertain_network_time: "some network transfer times are uncertain",
  partial_boundary: "partial edge buckets were excluded",
  resource_limit: "a recording limit was reached",
} as const;

function HistoricalUsageTable({
  data,
  previous,
  range,
  columns,
  sort,
  onSort,
  compare,
}: {
  data: HistoricalUsageReply;
  previous: HistoricalUsageReply | undefined;
  range: TimeRange;
  columns: readonly HistoricalUsageMetric[];
  sort: HistoricalUsageSort;
  onSort: (sort: HistoricalUsageSort) => void;
  compare: boolean;
}) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const units = useUnits();
  const apps = useMemo(
    () => (sort.dir === "asc" ? [...data.apps].reverse() : data.apps),
    [data.apps, sort.dir]
  );
  const { sorting, onSortingChange } = useColumnSort(sort, onSort, columns);
  const definitions = useMemo(
    () =>
      helper.columns([
        helper.accessor("name", {
          header: "App",
          enableSorting: false,
          meta: { align: "left" },
        }),
        ...columns.map((metric) =>
          helper.accessor((app) => gap(historicalValue(app.figures, metric)), {
            id: metric,
            header: HISTORICAL_LABELS[metric],
            meta: { align: "right" },
          })
        ),
      ]),
    [columns]
  );
  const { table, rows } = useDataTable({
    data: apps,
    columns: definitions,
    getRowId: appName,
    manualSorting: true,
    sorting,
    onSortingChange,
  });
  const prior = new Map(previous?.apps.map((app) => [app.name, app.figures]));
  const toggle = (name: string) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (!next.delete(name)) next.add(name);
      return next;
    });
  if (rows.length === 0)
    return (
      <UnrecordedRange>
        Attribution was not recorded for this interval.
      </UnrecordedRange>
    );
  return (
    <div className="overflow-x-auto">
      <Table aria-label="Recorded apps in this range">
        <TableHeader>
          <DataTableHead table={table} />
        </TableHeader>
        <DataTable
          table={table}
          rows={rows}
          renderRow={({ original: app }) => (
            <HistoricalAppRows
              key={app.name}
              name={app.name}
              figures={app.figures}
              details={app.process_details_available}
              previous={prior.get(app.name)}
              compare={compare}
              columns={columns}
              range={range}
              by={sort.by}
              units={units.bytes}
              open={expanded.has(app.name)}
              toggle={() => toggle(app.name)}
            />
          )}
          after={
            <TableRow className="hover:bg-transparent">
              <TableCell
                colSpan={columns.length + 1}
                className={`${APP_CELL} whitespace-normal text-[11px] text-muted-foreground`}
              >
                Up to ten recorded apps for{" "}
                {HISTORICAL_LABELS[sort.by].toLowerCase()}; reversing the order
                keeps those same apps. CPU power estimates CPU energy only.
              </TableCell>
            </TableRow>
          }
        />
      </Table>
    </div>
  );
}

const helper = columnHelper<HistoricalUsageApp>();
const appName = (app: HistoricalUsageApp) => app.name;

function visibleSort(
  sort: HistoricalUsageSort,
  by: HistoricalUsageMetric,
  columns: readonly HistoricalUsageMetric[]
): HistoricalUsageSort {
  if (columns.includes(sort.by)) return sort;
  return { by: columns.includes(by) ? by : (columns[0] ?? by), dir: "desc" };
}

function HistoricalAppRows({
  name,
  figures,
  details,
  previous,
  compare,
  columns,
  range,
  by,
  units,
  open,
  toggle,
}: {
  name: string;
  figures: HistoricalUsageFigures;
  details: boolean;
  previous: HistoricalUsageFigures | undefined;
  compare: boolean;
  columns: readonly HistoricalUsageMetric[];
  range: TimeRange;
  by: HistoricalUsageMetric;
  units: ReturnType<typeof useUnits>["bytes"];
  open: boolean;
  toggle: () => void;
}) {
  return (
    <>
      <TableRow className="text-fg-subtle">
        <TableCell className={APP_CELL}>
          <span className="inline-flex items-center gap-2 text-foreground">
            {details ? (
              <button
                type="button"
                aria-expanded={open}
                aria-label={`${open ? "Hide" : "Show"} ${name} recorded processes`}
                onClick={toggle}
                className="-ml-1 grid size-4 place-items-center rounded-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
              >
                <ChevronRight
                  aria-hidden
                  className={cn(
                    "size-3.5 transition-transform duration-(--motion-fast)",
                    open && "rotate-90"
                  )}
                />
              </button>
            ) : (
              <span aria-hidden className="-ml-1 inline-block size-4" />
            )}
            <InitialChip text={name} />
            <span>{name}</span>
          </span>
        </TableCell>
        <HistoricalUsageCells
          figures={figures}
          previous={previous}
          compare={compare}
          columns={columns}
          units={units}
          spanMs={range.toMs - range.fromMs}
        />
      </TableRow>
      {open && (
        <HistoricalUsageProcesses
          range={range}
          appName={name}
          by={by}
          columns={columns}
          units={units}
        />
      )}
    </>
  );
}

function comparisonRange(
  range: TimeRange | null,
  compare: boolean,
  count: number
): TimeRange | null {
  if (
    !range ||
    !compare ||
    count === 0 ||
    range.fromMs < range.toMs - range.fromMs
  )
    return null;
  return {
    fromMs: range.fromMs - (range.toMs - range.fromMs),
    toMs: range.fromMs,
  };
}
