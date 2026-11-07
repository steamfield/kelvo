import type { TimeRange } from "@core/brush";
import {
  formatBytes,
  formatClockSeconds,
  formatRate,
  MISSING,
  type RateUnits,
} from "@core/format";
import type { NetworkByApp } from "@core/generated/bindings";
import { USAGE_KEEP_MS } from "@core/generated/bindings";
import { networkByAppFailure } from "@core/history-state";
import { CommandFailure } from "@core/transport";
import type { OnChangeFn, SortingState } from "@tanstack/react-table";
import { memo, type ReactNode, useMemo, useState } from "react";
import {
  APP_CELL,
  AppsNote,
  AppsShell,
  OtherAppsRow,
  ShareCell,
  UnrecordedRange,
} from "~/components/app-table";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useDataTable,
} from "~/components/data-table";
import { HistoricalUsageCard } from "~/components/historical-usage-card";
import { SelectionChip } from "~/components/selection-chip";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";
import { heldFor, useOpenEdge, useRangeScope } from "~/hooks/use-range-scope";
import { useUnits } from "~/hooks/use-units";
import { cn } from "~/lib/utils";
import { useModuleRange } from "~/stores/chart-range";
import { InitialChip } from "~/widgets/initial-chip";
import {
  useCompleteEdge,
  useLatestBucket,
  useNetworkByApp,
} from "../_hooks/use-network-by-app";
import {
  type AppRow,
  appRows,
  appsTitle,
  firstRecordedMs,
  nowRates,
  openTailS,
  partialCoverage,
  remainderRows,
  unrecorded,
} from "../_lib/apps";

/**
 * Bytes per app over the selection, or over the chart window (D-091) when
 * nothing is selected (D-089). Totals come from `query_network_by_app`; "now" is
 * the latest complete 10 s bucket of the same command, so its names match.
 * The window ends where the engine's complete buckets end, not at the
 * newest sample: the bucket before it may still be waiting on the per-app
 * stream, and counting it would push its interface bytes into "System and
 * other".
 */
export function AppsCard({ windowMs }: { windowMs: number }) {
  const fixed = useModuleRange()?.endMs != null;
  return fixed || windowMs > USAGE_KEEP_MS ? (
    <MemoHistoricalNetworkApps windowMs={windowMs} />
  ) : (
    <LiveAppsCard windowMs={windowMs} />
  );
}

function HistoricalNetworkApps({ windowMs }: { windowMs: number }) {
  const { range } = useRangeScope(windowMs);
  return (
    <HistoricalUsageCard
      range={range}
      by="network"
      accent="net"
      title="Network by app"
    />
  );
}

const MemoHistoricalNetworkApps = memo(HistoricalNetworkApps);

function LiveAppsCard({ windowMs }: { windowMs: number }) {
  const fixed = useModuleRange()?.endMs != null;
  const complete = useCompleteEdge(useOpenEdge());
  const { selection, range, keepPrevious } = useRangeScope(windowMs, complete);
  const totals = useNetworkByApp(range, { keepPrevious });
  const latest = useLatestBucket(complete);
  const units = useUnits();
  const [sorting, setSorting] = useState<SortingState>([
    { id: "total", desc: true },
  ]);

  const data = heldFor(totals, windowMs);
  const now = useMemo(() => nowRates(latest.data), [latest.data]);
  const rows = useMemo(() => (data ? appRows(data, now) : []), [data, now]);
  const rest = useMemo(() => (data ? remainderRows(data) : []), [data]);
  const shown: TimeRange | null = data
    ? { fromMs: data.from_ms, toMs: data.to_ms }
    : selection;

  let body: ReactNode;
  if (totals.isError) {
    const error = totals.error;
    body = (
      <AppsNote>
        {error instanceof CommandFailure
          ? networkByAppFailure(error.error)
          : "Couldn't load app totals."}
      </AppsNote>
    );
  } else if (!data) {
    body = <AppsNote>Loading app totals…</AppsNote>;
  } else if (unrecorded(data)) {
    body = <Unrecorded data={data} edge={complete} />;
  } else {
    const partial = partialCoverage(data);
    const tail = openTailS(data);
    body = (
      <>
        {partial && (
          <AppsNote>
            Measured for <span className="figures">{partial.measuredS}</span> of{" "}
            <span className="figures">{partial.spanS}</span> s. Network history
            wasn't recording for the rest, so these totals cover{" "}
            <span className="figures">{partial.measuredS}</span> s.
          </AppsNote>
        )}
        {tail > 0 && (
          <AppsNote>
            The last <span className="figures">{tail}</span> s are still being
            measured. These totals update as they close.
          </AppsNote>
        )}
        {data.clamped && (
          <AppsNote>
            App totals here exceed the interface (bytes counted when their app
            was identified), so System and other reads 0 and shares are of the
            table's total.
          </AppsNote>
        )}
        <AppsTable
          rows={rows}
          rest={rest}
          units={units.rate}
          sorting={sorting}
          onSortingChange={setSorting}
        />
      </>
    );
  }

  return (
    <AppsShell
      title={appsTitle(selection || fixed ? (shown ?? range) : null, windowMs)}
      aside={selection && shown && <SelectionChip range={shown} />}
    >
      {body}
    </AppsShell>
  );
}

/**
 * A range with nothing recorded: a hatched box naming when Network history started, read from
 * the time after the range.
 */
function Unrecorded({
  data,
  edge,
}: {
  data: NetworkByApp;
  edge: number | null;
}) {
  const after =
    edge !== null && edge > data.to_ms
      ? { fromMs: data.to_ms, toMs: edge }
      : null;
  const later = useNetworkByApp(after);
  const started = later.data ? firstRecordedMs(later.data) : null;
  return (
    <UnrecordedRange>
      {started === null ? (
        "No app data for this range"
      ) : (
        <>
          No app data · Network history started{" "}
          <span className="figures">{formatClockSeconds(started)}</span>
        </>
      )}
    </UnrecordedRange>
  );
}

function AppsTable({
  rows,
  rest,
  units,
  sorting,
  onSortingChange,
}: {
  rows: readonly AppRow[];
  rest: readonly AppRow[];
  units: RateUnits;
  sorting: SortingState;
  onSortingChange: OnChangeFn<SortingState>;
}) {
  const [paused, setPaused] = useState(false);
  const { table, rows: sorted } = useDataTable({
    data: rows,
    columns: APP_COLUMNS,
    getRowId: appKey,
    tiebreak: byName,
    sorting,
    onSortingChange,
    hold: { paused },
  });
  const bytes = (n: number) => formatBytes(n);
  return (
    <div
      className="overflow-x-auto"
      onPointerEnter={() => setPaused(true)}
      onPointerLeave={() => setPaused(false)}
    >
      <Table aria-label="Network by app">
        <TableHeader>
          <DataTableHead table={table} />
        </TableHeader>
        <DataTable
          table={table}
          rows={sorted}
          renderRow={({ original: r }) => (
            <TableRow key={r.key} className="text-fg-subtle">
              <TableCell className={APP_CELL}>
                <span className="inline-flex items-center gap-2 text-foreground">
                  <InitialChip text={r.name} />
                  {r.name}
                </span>
              </TableCell>
              <TableCell className={cn(APP_CELL, "figures text-right")}>
                {bytes(r.rxBytes)}
              </TableCell>
              <TableCell className={cn(APP_CELL, "figures text-right")}>
                {bytes(r.txBytes)}
              </TableCell>
              <TableCell
                className={cn(APP_CELL, "figures text-right text-foreground")}
              >
                {bytes(r.totalBytes)}
              </TableCell>
              <ShareCell share={r.share} muted={false} />
              <TableCell
                className={cn(
                  APP_CELL,
                  "figures text-right",
                  !r.nowBps && "text-fg-faint"
                )}
              >
                {formatRate(r.nowBps, { units })}
              </TableCell>
            </TableRow>
          )}
          after={
            <>
              {rest.map((r, i) => (
                <OtherAppsRow
                  key={r.key}
                  name={r.name}
                  system={r.kind === "system"}
                  first={i === 0}
                >
                  <TableCell className={cn(APP_CELL, "figures text-right")}>
                    {bytes(r.rxBytes)}
                  </TableCell>
                  <TableCell className={cn(APP_CELL, "figures text-right")}>
                    {bytes(r.txBytes)}
                  </TableCell>
                  <TableCell className={cn(APP_CELL, "figures text-right")}>
                    {bytes(r.totalBytes)}
                  </TableCell>
                  <ShareCell share={r.share} muted />
                  <TableCell
                    className={cn(APP_CELL, "figures text-right text-fg-faint")}
                  >
                    {MISSING}
                  </TableCell>
                </OtherAppsRow>
              ))}
              <TableRow className="hover:bg-transparent">
                <TableCell
                  colSpan={6}
                  className="whitespace-normal px-3 pt-0 pb-2.5 pl-9 font-normal text-[11px] text-muted-foreground leading-[1.45]"
                >
                  macOS services (updates, backups, DNS). StatGlass can't see
                  these without a helper.
                </TableCell>
              </TableRow>
            </>
          }
        />
      </Table>
    </div>
  );
}

const helper = columnHelper<AppRow>();
const APP_COLUMNS = helper.columns([
  helper.accessor("name", {
    header: "App",
    sortFn: "text",
    meta: { align: "left" },
  }),
  helper.accessor("rxBytes", {
    id: "down",
    header: "Down",
    meta: { align: "right" },
  }),
  helper.accessor("txBytes", {
    id: "up",
    header: "Up",
    meta: { align: "right" },
  }),
  helper.accessor("totalBytes", {
    id: "total",
    header: "Total",
    meta: { align: "right" },
  }),
  helper.accessor((r) => gap(r.share), {
    id: "share",
    header: "Share",
    meta: { align: "right" },
  }),
  helper.accessor((r) => gap(r.nowBps), {
    id: "now",
    header: "Now",
    meta: { align: "right" },
  }),
]);
const appKey = (r: AppRow) => r.key;
const byName = (a: AppRow, b: AppRow) =>
  a.name.localeCompare(b.name) || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0);
