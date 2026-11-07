import { ratio } from "@core/chart-math";
import {
  type ByteUnits,
  fixed,
  formatBytes,
  formatPercent,
  formatRate,
  MISSING,
  type RateUnits,
} from "@core/format";
import type { SortingState } from "@tanstack/react-table";
import { type ReactNode, useMemo, useState } from "react";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useDataTable,
} from "~/components/data-table";
import type { ColumnDef } from "~/components/sort-header";
import { cn } from "~/lib/utils";
import { InitialChip } from "~/widgets/initial-chip";
import { MeterTrack } from "~/widgets/meter-track";
import {
  type CgroupColumn,
  type CgroupRow,
  type CgroupSort,
  DEFAULT_SORT,
  KIND_LABEL,
} from "../_lib/cgroups";

interface CellCtx {
  bytes: ByteUnits;
  rate: RateUnits;
}

const figure = (text: string, className?: string) => (
  <span className={cn("figures", className)}>{text}</span>
);

/** A share with an inline bar, as the process table draws % CPU. */
function Bar({
  fraction,
  fill,
  children,
}: {
  fraction: number | null;
  fill: string;
  children: ReactNode;
}) {
  return (
    <span className="inline-flex items-center gap-2">
      <MeterTrack
        fraction={fraction}
        fill={fill}
        className="inline-block w-12"
      />
      {children}
    </span>
  );
}

const COLUMNS: Record<CgroupColumn, ColumnDef<CgroupRow, CellCtx>> = {
  name: {
    label: "Name",
    align: "left",
    sortValue: (r) => r.name,
    cell: (r) => (
      <span className="inline-flex max-w-72 items-center gap-2 text-foreground">
        <InitialChip text={r.name} />
        <span className="truncate" title={r.name}>
          {r.name}
        </span>
      </span>
    ),
  },
  kind: {
    label: "Kind",
    align: "left",
    sortValue: (r) => KIND_LABEL[r.kind],
    cell: (r) => figure(KIND_LABEL[r.kind], "text-muted-foreground"),
  },
  cpu: {
    // Percent of one core, so the bar fills at one core and the figure can pass 100.
    label: "% CPU",
    align: "right",
    sortValue: (r) => r.cpu,
    cell: (r) => (
      <Bar fraction={ratio(r.cpu, 100)} fill="var(--color-cpu)">
        {figure(
          r.cpu === null ? MISSING : fixed(r.cpu, 1),
          "inline-block w-10 text-right text-foreground"
        )}
      </Bar>
    ),
  },
  mem: {
    label: "Memory",
    align: "right",
    sortValue: (r) => r.mem,
    cell: (r, c) => {
      const used = figure(formatBytes(r.mem, { units: c.bytes }));
      if (r.memMax === null) return used;
      return (
        <Bar fraction={ratio(r.mem, r.memMax)} fill="var(--color-mem)">
          <span className="inline-block min-w-30 text-right">
            {used}
            {figure(
              ` / ${formatBytes(r.memMax, { units: c.bytes })}`,
              "text-muted-foreground"
            )}
          </span>
        </Bar>
      );
    },
  },
  ioRead: {
    label: "Disk read",
    align: "right",
    sortValue: (r) => r.ioRead,
    cell: (r, c) => figure(formatRate(r.ioRead, { units: c.rate })),
  },
  ioWrite: {
    label: "Disk write",
    align: "right",
    sortValue: (r) => r.ioWrite,
    cell: (r, c) => figure(formatRate(r.ioWrite, { units: c.rate })),
  },
  memPsi: {
    label: "Memory pressure",
    align: "right",
    sortValue: (r) => r.memPsi,
    cell: (r) => figure(formatPercent(r.memPsi, { decimals: 1 })),
  },
};

const helper = columnHelper<CgroupRow>();
const getRowId = (row: CgroupRow) => row.id;
const byName = (a: CgroupRow, b: CgroupRow) => a.name.localeCompare(b.name);
const TABLE_COLUMNS = Object.fromEntries(
  Object.entries(COLUMNS).map(([id, column]) => [
    id,
    helper.accessor((row) => gap(column.sortValue(row)), {
      id,
      header: column.label,
      ...(column.align === "left" ? { sortFn: "text" as const } : {}),
      meta: {
        align: column.align,
        className: "h-auto whitespace-nowrap border-border border-b",
      },
    }),
  ])
);

const CELL = "whitespace-nowrap px-3 py-0 font-normal text-fg-subtle";

export interface CgroupTableProps {
  rows: readonly CgroupRow[];
  /** The folded remainder, pinned last whatever the sort. */
  other: { label: string; row: CgroupRow } | null;
  columns: readonly CgroupColumn[];
  sort: CgroupSort;
  onSort: (sort: CgroupSort) => void;
  bytes: ByteUnits;
  rate: RateUnits;
  empty: ReactNode;
}

/**
 * cgroup groups on a Linux host, styled like the process table. Row order
 * holds while the pointer is over the table, as on Processes; a sort the
 * user picks applies at once.
 */
export function CgroupTable({
  rows,
  other,
  columns,
  sort,
  onSort,
  bytes,
  rate,
  empty,
}: CgroupTableProps) {
  const [paused, setPaused] = useState(false);
  const activeSort = columns.includes(sort.by) ? sort : DEFAULT_SORT;
  const sorting = useMemo<SortingState>(
    () => [{ id: activeSort.by, desc: activeSort.dir === "desc" }],
    [activeSort.by, activeSort.dir]
  );
  const definitions = useMemo(
    () => helper.columns(columns.flatMap((id) => TABLE_COLUMNS[id] ?? [])),
    [columns]
  );
  const data = useDataTable({
    data: rows,
    columns: definitions,
    getRowId,
    tiebreak: byName,
    sorting,
    onSortingChange: (updater) => {
      const next = (
        typeof updater === "function" ? updater(sorting) : updater
      )[0];
      const by = columns.find((id) => id === next?.id);
      if (next && by) onSort({ by, dir: next.desc ? "desc" : "asc" });
    },
    hold: { paused, resetKey: `${activeSort.by}:${activeSort.dir}` },
  });
  const ctx: CellCtx = { bytes, rate };
  return (
    <div
      className="min-h-0 overflow-auto"
      onPointerEnter={() => setPaused(true)}
      onPointerLeave={() => setPaused(false)}
    >
      <table
        aria-label="Services and containers"
        className="w-full border-collapse text-[12px]"
      >
        <thead className="sticky top-0 z-10 bg-card">
          <DataTableHead table={data.table} />
        </thead>
        <DataTable
          table={data.table}
          rows={data.rows}
          className={
            other
              ? "[&_tr:last-child]:border-t"
              : rows.length > 0
                ? "[&_tr:last-child]:border-b"
                : undefined
          }
          renderRow={({ original: r }) => (
            <tr
              key={r.id}
              className="h-[29px] border-border-subtle border-b hover:bg-selected"
            >
              {columns.map((col) => (
                <td
                  key={col}
                  className={cn(
                    CELL,
                    COLUMNS[col].align === "right" && "text-right"
                  )}
                >
                  {COLUMNS[col].cell(r, ctx)}
                </td>
              ))}
            </tr>
          )}
          empty={
            <tr className="h-[29px]">
              <td
                colSpan={columns.length}
                className="px-3 py-0 text-muted-foreground"
              >
                {empty}
              </td>
            </tr>
          }
          after={
            other && (
              <OtherRow
                label={other.label}
                row={other.row}
                columns={columns}
                ctx={ctx}
              />
            )
          }
        />
      </table>
    </div>
  );
}

/** The remainder row, muted, as the per-app tables draw "Other apps". */
function OtherRow({
  label,
  row,
  columns,
  ctx,
}: {
  label: string;
  row: CgroupRow;
  columns: readonly CgroupColumn[];
  ctx: CellCtx;
}) {
  return (
    <tr className="h-[29px] border-border border-t text-muted-foreground">
      {columns.map((col) => (
        <td
          key={col}
          className={cn(
            CELL,
            "text-muted-foreground",
            COLUMNS[col].align === "right" && "text-right"
          )}
        >
          {col === "name" ? (
            <span className="inline-flex items-center gap-2">
              <span
                aria-hidden
                className="inline-block size-4 flex-none rounded-[4px] border border-border-strong/50 border-dashed"
              />
              <span className="truncate">{label}</span>
            </span>
          ) : col === "kind" || col === "memPsi" ? null : (
            COLUMNS[col].cell(row, ctx)
          )}
        </td>
      ))}
    </tr>
  );
}
