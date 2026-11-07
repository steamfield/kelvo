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
import { type ColumnDef, FlexRender } from "@tanstack/react-table";
import { type ReactNode, useMemo, useState } from "react";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  type TableFeatures,
  useColumnSort,
  useDataTable,
} from "~/components/data-table";
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

const helper = columnHelper<CgroupRow>();
const getRowId = (row: CgroupRow) => row.id;
const byName = (a: CgroupRow, b: CgroupRow) => a.name.localeCompare(b.name);

const STATIC_COLUMNS = {
  name: helper.accessor((r) => gap(r.name), {
    id: "name",
    header: "Name",
    sortFn: "text",
    meta: {
      align: "left",
      className: "h-auto whitespace-nowrap border-border border-b",
    },
    cell: ({ row: { original: r } }) => (
      <span className="inline-flex max-w-72 items-center gap-2 text-foreground">
        <InitialChip text={r.name} />
        <span className="truncate" title={r.name}>
          {r.name}
        </span>
      </span>
    ),
  }),
  kind: helper.accessor((r) => gap(KIND_LABEL[r.kind]), {
    id: "kind",
    header: "Kind",
    sortFn: "text",
    meta: {
      align: "left",
      className: "h-auto whitespace-nowrap border-border border-b",
    },
    cell: ({ row: { original: r } }) =>
      figure(KIND_LABEL[r.kind], "text-muted-foreground"),
  }),
  // Percent of one core, so the bar fills at one core and the figure can pass 100.
  cpu: helper.accessor((r) => gap(r.cpu), {
    id: "cpu",
    header: "% CPU",
    meta: {
      align: "right",
      className: "h-auto whitespace-nowrap border-border border-b",
    },
    cell: ({ row: { original: r } }) => (
      <Bar fraction={ratio(r.cpu, 100)} fill="var(--color-cpu)">
        {figure(
          r.cpu === null ? MISSING : fixed(r.cpu, 1),
          "inline-block w-10 text-right text-foreground"
        )}
      </Bar>
    ),
  }),
  memPsi: helper.accessor((r) => gap(r.memPsi), {
    id: "memPsi",
    header: "Memory pressure",
    meta: {
      align: "right",
      className: "h-auto whitespace-nowrap border-border border-b",
    },
    cell: ({ row: { original: r } }) =>
      figure(formatPercent(r.memPsi, { decimals: 1 })),
  }),
};

function cgroupColumns(c: CellCtx) {
  return {
    ...STATIC_COLUMNS,
    mem: helper.accessor((r) => gap(r.mem), {
      id: "mem",
      header: "Memory",
      meta: {
        align: "right",
        className: "h-auto whitespace-nowrap border-border border-b",
      },
      cell: ({ row: { original: r } }) => {
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
    }),
    ioRead: helper.accessor((r) => gap(r.ioRead), {
      id: "ioRead",
      header: "Disk read",
      meta: {
        align: "right",
        className: "h-auto whitespace-nowrap border-border border-b",
      },
      cell: ({ row: { original: r } }) =>
        figure(formatRate(r.ioRead, { units: c.rate })),
    }),
    ioWrite: helper.accessor((r) => gap(r.ioWrite), {
      id: "ioWrite",
      header: "Disk write",
      meta: {
        align: "right",
        className: "h-auto whitespace-nowrap border-border border-b",
      },
      cell: ({ row: { original: r } }) =>
        figure(formatRate(r.ioWrite, { units: c.rate })),
    }),
  };
}

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
  const { sorting, onSortingChange } = useColumnSort(
    activeSort,
    onSort,
    columns
  );
  const definitions = useMemo(() => {
    const definitions = cgroupColumns({ bytes, rate });
    return helper.columns(columns.map((id) => definitions[id]));
  }, [columns, bytes, rate]);
  const data = useDataTable({
    data: rows,
    columns: definitions,
    getRowId,
    tiebreak: byName,
    sorting,
    onSortingChange,
    hold: { paused, resetKey: `${activeSort.by}:${activeSort.dir}` },
  });
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
          renderRow={(row) => (
            <tr
              key={row.id}
              className="h-[29px] border-border-subtle border-b hover:bg-selected"
            >
              {row.getAllCells().map((cell) => (
                <td
                  key={cell.id}
                  className={cn(
                    CELL,
                    cell.column.columnDef.meta?.align === "right" &&
                      "text-right"
                  )}
                >
                  <FlexRender cell={cell} />
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
                columns={definitions}
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
}: {
  label: string;
  row: CgroupRow;
  columns: readonly ColumnDef<TableFeatures, CgroupRow>[];
}) {
  const data = useMemo(() => [row], [row]);
  const model = useDataTable({ data, columns, getRowId });
  return (
    <tr className="h-[29px] border-border border-t text-muted-foreground">
      {model.rows
        .flatMap((row) => row.getAllCells())
        .map((cell) => (
          <td
            key={cell.id}
            className={cn(
              CELL,
              "text-muted-foreground",
              cell.column.columnDef.meta?.align === "right" && "text-right"
            )}
          >
            {cell.column.id === "name" ? (
              <span className="inline-flex items-center gap-2">
                <span
                  aria-hidden
                  className="inline-block size-4 flex-none rounded-[4px] border border-border-strong/50 border-dashed"
                />
                <span className="truncate">{label}</span>
              </span>
            ) : cell.column.id === "kind" ||
              cell.column.id === "memPsi" ? null : (
              <FlexRender cell={cell} />
            )}
          </td>
        ))}
    </tr>
  );
}
