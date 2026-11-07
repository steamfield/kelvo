import { ratio } from "@core/chart-math";
import { type ByteUnits, formatBytes } from "@core/format";
import { type ReactNode, useMemo } from "react";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useDataTable,
} from "~/components/data-table";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";
import { MeterTrack } from "~/widgets/meter-track";

export interface VolumeRow {
  /** `vol` label value and row key. */
  id: string;
  /** "Macintosh HD", "Data". */
  name: string;
  usedBytes: number | null;
  freeBytes: number | null;
  totalBytes: number | null;
  disconnected?: boolean;
}

export interface VolumeTableProps {
  /** Boot container first. */
  rows: readonly VolumeRow[];
  units: ByteUnits;
  /** A trailing control per row (Analyze, Scan). */
  action?: ((row: VolumeRow) => ReactNode) | undefined;
}

/**
 * Disk volumes (v1-local-monitor.md 4.12): used, free, size and a
 * usage bar in the Disk accent.
 */
export function VolumeTable({ rows, units, action }: VolumeTableProps) {
  const hasAction = Boolean(action);
  const columns = useMemo(
    () =>
      helper.columns([
        helper.accessor("name", {
          header: "Volume",
          sortFn: "text",
          meta: { align: "left" },
        }),
        helper.accessor((r) => gap(r.usedBytes), {
          id: "used",
          header: "Used",
          meta: { align: "right" },
        }),
        helper.accessor((r) => gap(r.freeBytes), {
          id: "free",
          header: "Free",
          meta: { align: "right" },
        }),
        helper.accessor((r) => gap(r.totalBytes), {
          id: "size",
          header: "Size",
          meta: { align: "right" },
        }),
        helper.display({
          id: "usage",
          header: () => <span className="sr-only">Usage</span>,
          meta: { align: "left", className: "w-40" },
        }),
        ...(hasAction
          ? [
              helper.display({
                id: "actions",
                header: () => <span className="sr-only">Actions</span>,
                meta: { align: "left", className: "w-0" },
              }),
            ]
          : []),
      ]),
    [hasAction]
  );
  const data = useDataTable({ data: rows, columns, getRowId });
  return (
    <Table>
      <TableHeader>
        <DataTableHead table={data.table} />
      </TableHeader>
      <DataTable
        table={data.table}
        rows={data.rows}
        renderRow={({ original: r }) => {
          const frac = ratio(r.usedBytes, r.totalBytes);
          return (
            <TableRow key={r.id} className="text-[12px]">
              <TableCell className="text-foreground">
                {r.name}
                {r.disconnected && (
                  <span className="ml-2 text-muted-foreground">
                    Disconnected
                  </span>
                )}
              </TableCell>
              <TableCell className="figures text-right">
                {formatBytes(r.usedBytes, { units })}
              </TableCell>
              <TableCell className="figures text-right">
                {formatBytes(r.freeBytes, { units })}
              </TableCell>
              <TableCell className="figures text-right text-fg-subtle">
                {formatBytes(r.totalBytes, { units })}
              </TableCell>
              <TableCell>
                <MeterTrack fraction={frac} fill="var(--color-disk)" />
              </TableCell>
              {action && (
                <TableCell className="whitespace-nowrap text-right">
                  {action(r)}
                </TableCell>
              )}
            </TableRow>
          );
        }}
      />
    </Table>
  );
}

const helper = columnHelper<VolumeRow>();
const getRowId = (row: VolumeRow) => row.id;
