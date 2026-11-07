import { formatRate } from "@core/format";
import { useMemo } from "react";
import { Link } from "react-router";
import { AppsNote, AppsShell } from "~/components/app-table";
import {
  columnHelper,
  DataTable,
  DataTableHead,
  useDataTable,
} from "~/components/data-table";
import { Table, TableCell, TableHeader, TableRow } from "~/components/ui/table";
import {
  topRowsView,
  useProcessInterest,
  useProcessRows,
} from "~/hooks/use-process-interest";
import { useUnits } from "~/hooks/use-units";
import { InitialChip } from "~/widgets/initial-chip";
import { measuredTraffic } from "../_lib/network";

const ROWS = 12;
interface NowRow {
  pid: number;
  name: string;
  bps: number;
}
const helper = columnHelper<NowRow>();
const columns = helper.columns([
  helper.accessor("name", {
    header: "App",
    enableSorting: false,
    meta: { align: "left" },
  }),
  helper.accessor("bps", {
    header: "Now",
    meta: { align: "right", ranked: true },
  }),
]);
const processId = (row: NowRow) => String(row.pid);

/**
 * "Network history off": no totals and no brush, only live rates
 * per process (D-081), with a way back to the setting.
 */
export function AppsNowCard() {
  useProcessInterest(topRowsView({ by: "netTotal", dir: "desc" }, ROWS, true));
  const live = useProcessRows();
  const units = useUnits();
  const rows = useMemo(
    () =>
      live === null
        ? []
        : measuredTraffic(live)
            .map((p) => ({
              pid: p.pid,
              name: p.name,
              bps: (p.net_rx_bps ?? 0) + (p.net_tx_bps ?? 0),
            }))
            .sort((a, b) => b.bps - a.bps)
            .slice(0, ROWS),
    [live]
  );
  const data = useDataTable({
    data: rows,
    columns,
    getRowId: processId,
    manualSorting: true,
  });

  return (
    <AppsShell title="Apps, now">
      <AppsNote>
        Network history is off, so there are no totals and the chart has no
        brush. Live rates still show.{" "}
        <Link
          to="/dashboard/settings"
          className="rounded-sm text-link outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring"
        >
          Turn it on in Settings
        </Link>
      </AppsNote>
      <Table aria-label="Network by app, now">
        <TableHeader>
          <DataTableHead table={data.table} />
        </TableHeader>
        <DataTable
          table={data.table}
          rows={data.rows}
          empty={
            <TableRow className="hover:bg-transparent">
              <TableCell
                colSpan={2}
                className="px-3 py-2 font-normal text-[12px] text-muted-foreground"
              >
                {live === null
                  ? "Measuring network by process…"
                  : "None of your processes sent or received anything."}
              </TableCell>
            </TableRow>
          }
          renderRow={({ original: r }) => (
            <TableRow key={r.pid} className="text-[12px]">
              <TableCell className="px-3 py-1.75 text-foreground">
                <span className="inline-flex items-center gap-2">
                  <InitialChip text={r.name} />
                  {r.name}
                </span>
              </TableCell>
              <TableCell className="figures px-3 py-1.75 text-right text-foreground">
                {formatRate(r.bps, { units: units.rate })}
              </TableCell>
            </TableRow>
          )}
        />
      </Table>
    </AppsShell>
  );
}
