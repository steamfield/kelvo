import type { RowData, Table } from "@tanstack/react-table";
import type { ReactNode } from "react";
import { TableRow } from "~/components/ui/table";
import type { TableFeatures } from "./features";
import { SortHeader } from "./sort-header";

/** Header rows only; the owner chooses the thead's position and surface. */
export function DataTableHead<T extends RowData>({
  table,
  leading,
  trailing,
}: {
  table: Table<TableFeatures, T>;
  leading?: ReactNode;
  trailing?: ReactNode;
}) {
  return table.getHeaderGroups().map((group) => (
    <TableRow
      key={group.id}
      className="border-0 transition-none hover:bg-transparent"
    >
      {leading}
      {group.headers.map((header) => (
        <SortHeader key={header.id} header={header} />
      ))}
      {trailing}
    </TableRow>
  ));
}
