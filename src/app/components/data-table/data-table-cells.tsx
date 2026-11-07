import { FlexRender, type Row, type RowData } from "@tanstack/react-table";
import { TableCell } from "~/components/ui/table";
import { cn } from "~/lib/utils";
import type { TableFeatures } from "./features";

export function DataTableCells<T extends RowData>({
  row,
  className,
}: {
  row: Row<TableFeatures, T>;
  className?: string;
}) {
  return row.getAllCells().map((cell) => (
    <TableCell
      key={cell.id}
      className={cn(
        className,
        cell.column.columnDef.meta?.align === "right" && "text-right",
        cell.column.columnDef.meta?.cellClassName
      )}
    >
      <FlexRender cell={cell} />
    </TableCell>
  ));
}
