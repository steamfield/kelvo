import { formatBytes, formatRate, type RateUnits } from "@core/format";
import type { SortingState } from "@tanstack/react-table";
import { useState } from "react";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  gap,
  useDataTable,
} from "~/components/data-table";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";

export interface InterfaceRow {
  /** BSD name and row key, "en0". */
  id: string;
  /** "Wi‑Fi", "Ethernet", "VPN", "Bridge". */
  kind: string;
  /** `net.rx{iface}` / `net.tx{iface}`, bytes per second; `null` is a gap. */
  rxBps: number | null;
  txBps: number | null;
  /** Received and sent since boot, bytes. */
  rxBytes: number | null;
  txBytes: number | null;
  /** The interface left the layout (cable pulled, VPN down). */
  disconnected?: boolean;
}

export interface InterfaceTableProps {
  rows: readonly InterfaceRow[];
  units: RateUnits;
}

/**
 * Network interfaces (v1-local-monitor.md 4.11; styled like the CPU page's
 * process table). No IP addresses: they are personal data in screenshots.
 */
export function InterfaceTable({ rows, units }: InterfaceTableProps) {
  const [paused, setPaused] = useState(false);
  const [sorting, setSorting] = useState<SortingState>([]);
  const data = useDataTable({
    data: rows,
    columns,
    getRowId,
    sorting,
    onSortingChange: setSorting,
    hold: { paused },
  });
  return (
    <Table
      onPointerEnter={() => setPaused(true)}
      onPointerLeave={() => setPaused(false)}
    >
      <TableHeader>
        <DataTableHead table={data.table} />
      </TableHeader>
      <DataTable
        table={data.table}
        rows={data.rows}
        renderRow={({ original: r }) => (
          <TableRow key={r.id} className="text-[12px]">
            <TableCell className="figures text-foreground">{r.id}</TableCell>
            <TableCell className="font-normal text-fg-subtle">
              {r.disconnected ? (
                <span className="text-muted-foreground">Disconnected</span>
              ) : (
                r.kind
              )}
            </TableCell>
            <TableCell className="figures text-right">
              {formatRate(r.rxBps, { units })}
            </TableCell>
            <TableCell className="figures text-right">
              {formatRate(r.txBps, { units })}
            </TableCell>
            <TableCell className="figures text-right text-fg-subtle">
              {formatBytes(r.rxBytes)}
            </TableCell>
            <TableCell className="figures text-right text-fg-subtle">
              {formatBytes(r.txBytes)}
            </TableCell>
          </TableRow>
        )}
      />
    </Table>
  );
}

const helper = columnHelper<InterfaceRow>();
const getRowId = (row: InterfaceRow) => row.id;
const columns = helper.columns([
  helper.accessor("id", {
    header: "Interface",
    sortFn: "text",
    meta: { align: "left" },
  }),
  helper.accessor("kind", {
    header: "Kind",
    sortFn: "text",
    meta: { align: "left" },
  }),
  helper.accessor((r) => gap(r.rxBps), {
    id: "down",
    header: "Down",
    meta: { align: "right" },
  }),
  helper.accessor((r) => gap(r.txBps), {
    id: "up",
    header: "Up",
    meta: { align: "right" },
  }),
  helper.accessor((r) => gap(r.rxBytes), {
    id: "received",
    header: "Received",
    meta: { align: "right" },
  }),
  helper.accessor((r) => gap(r.txBytes), {
    id: "sent",
    header: "Sent",
    meta: { align: "right" },
  }),
]);
