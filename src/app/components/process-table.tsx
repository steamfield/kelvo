import { ratio } from "@core/chart-math";
import {
  type ByteUnits,
  fixed,
  formatBytes,
  formatRate,
  MISSING,
  type RateUnits,
} from "@core/format";
import type { LiveProcess } from "@core/generated/bindings";
import { holdOrder } from "@core/held-order";
import { targetKey } from "@core/process-selection";
import {
  type SortingState,
  sortFn_textCaseSensitive,
} from "@tanstack/react-table";
import type { Virtualizer } from "@tanstack/react-virtual";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useDataTable,
} from "~/components/data-table";
import {
  ProcessSelectionHead,
  ProcessSelectionRow,
  ProcessSelectionSurface,
  SELECTION_BAR_PX,
  SharedProcessActions,
  useProcessSelection,
} from "~/components/process-selection";
import type { ColumnDef } from "~/components/sort-header";
import { Table, TableCell, TableRow } from "~/components/ui/table";
import { cn } from "~/lib/utils";
import { InitialChip } from "~/widgets/initial-chip";
import { MeterTrack } from "~/widgets/meter-track";

export type ProcessColumn =
  | "name"
  | "pid"
  | "port"
  | "cpu"
  | "mem"
  | "compressed"
  | "threads"
  | "wakeups"
  | "energy"
  | "diskRead"
  | "diskWrite"
  | "diskTotal"
  | "netRx"
  | "netTx"
  | "netTotal"
  | "gpu"
  | "user";

export interface ProcessSort {
  by: ProcessColumn;
  dir: "asc" | "desc";
}

export interface ProcessTableProps {
  rows: readonly LiveProcess[];
  selectable?: boolean;
  columns: readonly ProcessColumn[];
  sort: ProcessSort;
  onSort: (sort: ProcessSort) => void;
  /** % CPU that fills the inline bar. The CPU page scales to 140%. */
  cpuBarMaxPct?: number;
  /** Viewport height of the scrolling body, px. Rows past it are virtualized. */
  height: number;
  memUnits?: ByteUnits;
  rateUnits?: RateUnits;
  /**
   * Keep row order while the pointer is over the table so a row does not move
   * under the cursor; values keep updating, new rows go last, and order
   * resumes when the pointer leaves (plan 4.14). Sorting re-orders at once.
   */
  freezeOrderOnHover?: boolean;
  /**
   * Size the box to the most rows seen while mounted, up to `height`, rather
   * than to the rows now: a table whose rows come and go at 1 Hz (idle
   * processes dropping out) grows but never shrinks. The mark lives as long
   * as the table, so a caller keeps the table mounted through empty batches
   * and passes `empty` rather than swapping in its own message.
   */
  growOnly?: boolean;
  /** One row's message while there are no rows ("Measuring…"). */
  empty?: ReactNode;
  ariaLabel: string;
}

/** Row height in px: 12 px text plus 7 px padding top and bottom, plus a hairline. */
const ROW_H = 29;

/**
 * Read plus write, bytes per second, as Rust ranks `disk_total`
 * (`procview.rs`); null when either is not a number.
 */
export function diskTotal(p: LiveProcess): number | null {
  if (p.disk_read_bps == null || p.disk_write_bps == null) return null;
  return p.disk_read_bps + p.disk_write_bps;
}

/**
 * Received plus sent, bytes per second; null when the row carries no
 * network rates (not sampled, D-081).
 */
export function netTotal(p: LiveProcess): number | null {
  if (p.net_rx_bps == null || p.net_tx_bps == null) return null;
  return p.net_rx_bps + p.net_tx_bps;
}

/**
 * `sorted` in the order of `keys` (the order when the pointer entered), with
 * rows that have since appeared appended in sorted order and rows that have
 * gone dropped.
 */
export function frozenOrder(
  sorted: readonly LiveProcess[],
  keys: readonly string[]
): LiveProcess[] {
  return holdOrder(sorted, keys, processKey);
}

/** Stable identity: a pid is reused, (pid, start time) is not. */
export const processKey = targetKey;

/** Sort a copy; `null` values go last in either direction, ties by pid. */
export function sortProcesses(
  rows: readonly LiveProcess[],
  { by, dir }: ProcessSort
): LiveProcess[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...rows].sort((a, b) => {
    const value = COLUMNS[by].sortValue;
    const va = value(a);
    const vb = value(b);
    if (va == null && vb == null) return a.pid - b.pid;
    if (va == null) return 1;
    if (vb == null) return -1;
    if (va < vb) return -sign;
    if (va > vb) return sign;
    return a.pid - b.pid;
  });
}

const grouped = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

/** Ports listed in a cell before the rest collapse to "+N". */
const PORTS_SHOWN = 3;

/**
 * "3000, 5173", or "5432, 6379, 8080 +4" past three. Blank for a process
 * listening on none, which is most rows; a dash only when ports were not read.
 */
export function portsText(ports: readonly number[] | null): string {
  if (ports == null) return MISSING;
  const head = ports.slice(0, PORTS_SHOWN).join(", ");
  const rest = ports.length - PORTS_SHOWN;
  return rest > 0 ? `${head} +${rest}` : head;
}

function num(v: number | null, decimals = 0): string {
  if (v == null || !Number.isFinite(v)) return MISSING;
  return decimals === 0 ? grouped.format(v) : fixed(v, decimals);
}

/** What a process cell reads besides its row. */
interface CellCtx {
  cpuBarMaxPct: number;
  memUnits: ByteUnits;
  rateUnits: RateUnits;
}

const figure = (text: string, className?: string) => (
  <span className={cn("figures", className)}>{text}</span>
);

/** A percent with an inline bar in the module color, the bar full at `max`. */
function PercentBar({
  pct,
  max,
  fill,
}: {
  pct: number | null;
  max: number;
  fill: string;
}) {
  return (
    <span className="inline-flex items-center gap-2">
      <MeterTrack
        fraction={ratio(pct, max)}
        fill={fill}
        className="inline-block w-12"
      />
      <span className="figures inline-block w-10 text-right text-foreground">
        {num(pct, 1)}
      </span>
    </span>
  );
}

const COLUMNS: Record<ProcessColumn, ColumnDef<LiveProcess, CellCtx>> = {
  name: {
    label: "Process",
    align: "left",
    sortValue: (p) => p.name.toLowerCase(),
    cell: (p) => (
      <span className="inline-flex max-w-64 items-center gap-2 text-foreground">
        <InitialChip text={p.name} />
        <span className="truncate">{p.name}</span>
      </span>
    ),
  },
  pid: {
    label: "PID",
    align: "right",
    sortValue: (p) => p.pid,
    cell: (p) => figure(String(p.pid), "text-muted-foreground"),
  },
  port: {
    label: "Port",
    align: "right",
    // The lowest port; a process listening on none sorts with the unread.
    sortValue: (p) => p.ports?.[0] ?? null,
    cell: (p) => (
      <span
        className="figures text-muted-foreground"
        title={
          p.ports && p.ports.length > PORTS_SHOWN
            ? p.ports.join(", ")
            : undefined
        }
      >
        {portsText(p.ports)}
      </span>
    ),
  },
  cpu: {
    label: "% CPU",
    align: "right",
    sortValue: (p) => p.cpu_pct,
    cell: (p, c) => (
      <PercentBar
        pct={p.cpu_pct}
        max={c.cpuBarMaxPct}
        fill="var(--color-cpu)"
      />
    ),
  },
  mem: {
    label: "Memory",
    align: "right",
    sortValue: (p) => p.mem_bytes,
    cell: (p, c) => figure(formatBytes(p.mem_bytes, { units: c.memUnits })),
  },
  compressed: {
    label: "Compressed",
    align: "right",
    sortValue: (p) => p.compressed_bytes,
    cell: (p, c) =>
      figure(formatBytes(p.compressed_bytes, { units: c.memUnits })),
  },
  threads: {
    label: "Threads",
    align: "right",
    sortValue: (p) => p.threads,
    cell: (p) => figure(num(p.threads)),
  },
  wakeups: {
    label: "Idle wake-ups",
    align: "right",
    sortValue: (p) => p.idle_wakeups_per_s,
    cell: (p) => figure(num(p.idle_wakeups_per_s)),
  },
  energy: {
    label: "Energy",
    align: "right",
    sortValue: (p) => p.energy,
    cell: (p) => figure(num(p.energy, 1)),
  },
  diskRead: {
    label: "Disk read",
    align: "right",
    sortValue: (p) => p.disk_read_bps,
    cell: (p, c) => figure(formatRate(p.disk_read_bps, { units: c.rateUnits })),
  },
  diskWrite: {
    label: "Disk write",
    align: "right",
    sortValue: (p) => p.disk_write_bps,
    cell: (p, c) =>
      figure(formatRate(p.disk_write_bps, { units: c.rateUnits })),
  },
  diskTotal: {
    label: "Disk total",
    align: "right",
    sortValue: diskTotal,
    cell: (p, c) =>
      figure(
        formatRate(diskTotal(p), { units: c.rateUnits }),
        "text-foreground"
      ),
  },
  netRx: {
    label: "Down",
    align: "right",
    sortValue: (p) => p.net_rx_bps,
    cell: (p, c) => figure(formatRate(p.net_rx_bps, { units: c.rateUnits })),
  },
  netTx: {
    label: "Up",
    align: "right",
    sortValue: (p) => p.net_tx_bps,
    cell: (p, c) => figure(formatRate(p.net_tx_bps, { units: c.rateUnits })),
  },
  netTotal: {
    label: "Net total",
    align: "right",
    sortValue: netTotal,
    cell: (p, c) =>
      figure(
        formatRate(netTotal(p), { units: c.rateUnits }),
        "text-foreground"
      ),
  },
  gpu: {
    label: "% GPU",
    align: "right",
    sortValue: (p) => p.gpu_pct,
    // Percent of the whole GPU (D-085), so the bar is out of 100.
    cell: (p) => (
      <PercentBar pct={p.gpu_pct} max={100} fill="var(--color-gpu)" />
    ),
  },
  user: {
    label: "User",
    align: "left",
    sortValue: (p) => p.user,
    cell: (p) => figure(p.user, "text-muted-foreground"),
  },
};

/**
 * Sortable, virtualized process table. The external sort stays in the process
 * vocabulary; the shared table owns sorting and hover order.
 */
export function ProcessTable(props: ProcessTableProps) {
  const { rows, columns, sort, onSort } = props;
  const [paused, setPaused] = useState(false);
  const { by, dir } = sort;
  const sorting = useMemo<SortingState>(
    () => [{ id: by, desc: dir === "desc" }],
    [by, dir]
  );
  const definitions = useMemo(
    () =>
      helper.columns(
        columns.map((id) => {
          const column = COLUMNS[id];
          return helper.accessor((row) => gap(column.sortValue(row)), {
            id,
            header: column.label,
            sortFn:
              id === "user"
                ? sortFn_textCaseSensitive
                : column.align === "left"
                  ? "text"
                  : "auto",
            meta: {
              align: column.align,
              className: "h-auto whitespace-nowrap border-border border-b",
            },
          });
        })
      ),
    [columns]
  );
  const data = useDataTable({
    data: rows,
    columns: definitions,
    getRowId: processKey,
    tiebreak: byPid,
    sorting,
    onSortingChange: (updater) => {
      const next = (
        typeof updater === "function" ? updater(sorting) : updater
      )[0];
      const column = columns.find((id) => id === next?.id);
      if (next && column)
        onSort({ by: column, dir: next.desc ? "desc" : "asc" });
    },
    hold: { paused, resetKey: `${by}:${dir}` },
  });
  const descriptors = useMemo(
    () =>
      data.rows.map(({ id, original: p }) => ({
        key: id,
        label: `${p.name} (${p.pid})`,
        targets: [p],
      })),
    [data.rows]
  );
  const body = (
    <ProcessTableBody
      {...props}
      data={data}
      setPaused={setPaused}
      height={props.height + (props.selectable ? SELECTION_BAR_PX : 0)}
    />
  );
  if (!props.selectable) return body;
  return (
    <ProcessSelectionSurface
      rows={descriptors}
      allTargets={rows}
      orderKey={`${by}:${dir}`}
      height={props.height + ROW_H + 4 + SELECTION_BAR_PX}
    >
      {body}
    </ProcessSelectionSurface>
  );
}

const helper = columnHelper<LiveProcess>();
const byPid = (a: LiveProcess, b: LiveProcess) => a.pid - b.pid;

function ProcessTableBody({
  columns,
  cpuBarMaxPct = 100,
  height,
  memUnits = "GB",
  rateUnits = "MBps",
  freezeOrderOnHover = false,
  growOnly = false,
  empty,
  ariaLabel,
  data,
  setPaused,
}: ProcessTableProps & {
  data: ReturnType<typeof useDataTable<LiveProcess>>;
  setPaused: (paused: boolean) => void;
}) {
  const selection = useProcessSelection();
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useRef<Virtualizer<HTMLDivElement, Element>>(null);
  useEffect(() => () => setPaused(false), [setPaused]);
  const byKey = new Map(data.rows.map((row) => [row.id, row]));
  const rows = selection
    ? selection.rows.flatMap((r) => {
        const row = byKey.get(r.key);
        return row ? [row] : [];
      })
    : data.rows;
  const hasActions = Boolean(selection);
  const colSpan = columns.length + (hasActions ? 2 : 0);
  const showEmpty = rows.length === 0 && empty !== undefined;
  const bodyPx = Math.min((showEmpty ? 1 : rows.length) * ROW_H, height);
  const [tallestPx, setTallestPx] = useState(bodyPx);
  if (growOnly && bodyPx > tallestPx) setTallestPx(bodyPx);
  const revealRef = selection?.reveal;
  useEffect(() => {
    if (!revealRef) return;
    revealRef.current = (key) => {
      const index = rows.findIndex((row) => row.id === key);
      if (index >= 0) virtualizer.current?.scrollToIndex(index);
    };
    return () => {
      revealRef.current = null;
    };
  }, [revealRef, rows]);
  const ctx: CellCtx = { cpuBarMaxPct, memUnits, rateUnits };
  return (
    <div
      ref={scrollRef}
      data-testid="process-table-scroll"
      data-process-scroll
      className="min-h-0 overflow-auto"
      style={
        growOnly
          ? { height: Math.max(tallestPx, bodyPx) + ROW_H + 4 }
          : { maxHeight: height + ROW_H + 4 }
      }
      onPointerEnter={freezeOrderOnHover ? () => setPaused(true) : undefined}
      onPointerLeave={freezeOrderOnHover ? () => setPaused(false) : undefined}
    >
      <Table
        aria-label={ariaLabel}
        className="w-full border-collapse text-[12px]"
      >
        <thead className="sticky top-0 z-10 bg-card">
          <DataTableHead
            table={data.table}
            leading={<ProcessSelectionHead />}
            trailing={
              hasActions && (
                <th
                  scope="col"
                  className="w-36 border-border border-b p-0 font-normal"
                >
                  <span className="sr-only">Actions</span>
                </th>
              )
            }
          />
        </thead>
        <DataTable
          table={data.table}
          rows={rows}
          className={
            rows.length > 0
              ? "[&_tr:last-child]:border-b [&_tr[aria-hidden]]:border-0"
              : undefined
          }
          virtual={{
            scrollRef,
            estimateSize: ROW_H,
            overscan: 6,
            initialHeight: height,
            enabled: true,
            ref: virtualizer,
          }}
          empty={
            showEmpty && (
              <TableRow className="h-[29px] border-0 transition-none hover:bg-transparent">
                <TableCell
                  colSpan={colSpan}
                  className="px-3 py-0 text-muted-foreground"
                >
                  {empty}
                </TableCell>
              </TableRow>
            )
          }
          renderRow={({ original: p, id }) => (
            <ProcessSelectionRow
              rowKey={id}
              label={`${p.name} (${p.pid})`}
              targets={[p]}
              data-key={id}
              className="group h-[29px] border-border-subtle border-b hover:bg-selected"
            >
              {columns.map((col) => (
                <td
                  key={col}
                  className={cn(
                    "whitespace-nowrap px-3 py-0 font-normal text-fg-subtle",
                    COLUMNS[col].align === "right" && "text-right"
                  )}
                >
                  {COLUMNS[col].cell(p, ctx)}
                </td>
              ))}
              {hasActions && (
                <td className="w-36 min-w-36 whitespace-nowrap py-0 pr-2 pl-1 text-right">
                  <span className="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 group-data-[state=open]:opacity-100">
                    <SharedProcessActions targets={[p]} />
                  </span>
                </td>
              )}
            </ProcessSelectionRow>
          )}
        />
      </Table>
    </div>
  );
}
