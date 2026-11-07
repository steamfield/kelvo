import type { Row, RowData, Table } from "@tanstack/react-table";
import {
  observeElementRect,
  useVirtualizer,
  type Virtualizer,
} from "@tanstack/react-virtual";
import {
  Fragment,
  type ReactNode,
  type Ref,
  type RefObject,
  useImperativeHandle,
} from "react";
import { TableBody, TableCell, TableRow } from "~/components/ui/table";
import { DataTableCells } from "./data-table-cells";
import type { TableFeatures } from "./features";

export interface DataTableVirtual {
  scrollRef: RefObject<HTMLDivElement | null>;
  estimateSize: number;
  overscan: number;
  initialHeight: number;
  enabled: boolean;
  ref?: Ref<Virtualizer<HTMLDivElement, Element>>;
}

/** Body only, so selection and grouped bodies can share the same table model. */
interface DataTableProps<T extends RowData> {
  className?: string | undefined;
  table: Table<TableFeatures, T>;
  rows: readonly Row<TableFeatures, T>[];
  renderRow?: (row: Row<TableFeatures, T>) => ReactNode;
  after?: ReactNode;
  /** A complete empty-state row, including its cells. */
  empty?: ReactNode;
  virtual?: DataTableVirtual;
}

export function DataTable<T extends RowData>(props: DataTableProps<T>) {
  return props.virtual ? (
    <VirtualBody {...props} virtual={props.virtual} />
  ) : (
    <TableBody className={props.className}>
      <BodyRows {...props} />
      {props.after}
    </TableBody>
  );
}

function VirtualBody<T extends RowData>({
  table,
  rows,
  renderRow,
  after,
  empty,
  virtual,
  className,
}: DataTableProps<T> & { virtual: DataTableVirtual }) {
  const virtualizer = useVirtualizer({
    enabled: virtual?.enabled ?? false,
    count: rows.length,
    getScrollElement: () => virtual?.scrollRef.current ?? null,
    estimateSize: () => virtual?.estimateSize ?? 29,
    getItemKey: (index) => rows[index]?.id ?? index,
    overscan: virtual?.overscan ?? 6,
    initialRect: { width: 0, height: virtual?.initialHeight ?? 0 },
    // Hidden windows and test DOMs measure zero until layout is available.
    observeElementRect: (instance, cb) =>
      observeElementRect(instance, (rect) =>
        cb(
          rect.height > 0
            ? rect
            : { ...rect, height: virtual?.initialHeight ?? 0 }
        )
      ),
  });
  useImperativeHandle(virtual?.ref, () => virtualizer, [virtualizer]);
  const items = virtualizer.getVirtualItems();
  const padTop = items[0]?.start ?? 0;
  const padBottom = virtualizer.getTotalSize() - (items.at(-1)?.end ?? 0);
  const visible = virtual?.enabled
    ? items.flatMap((item) => {
        const row = rows[item.index];
        return row ? [row] : [];
      })
    : rows;
  const colSpan = table.getAllLeafColumns().length;
  return (
    <TableBody className={className}>
      {virtual?.enabled && padTop > 0 && (
        <Spacer height={padTop} colSpan={colSpan} />
      )}
      <BodyRows
        table={table}
        rows={visible}
        {...(renderRow ? { renderRow } : {})}
        empty={rows.length === 0 && empty}
      />
      {virtual?.enabled && padBottom > 0 && (
        <Spacer height={padBottom} colSpan={colSpan} />
      )}
      {after}
    </TableBody>
  );
}

function BodyRows<T extends RowData>({
  rows,
  renderRow,
  empty,
}: DataTableProps<T>) {
  return (
    <>
      {rows.map((row) =>
        renderRow ? (
          <Fragment key={row.id}>{renderRow(row)}</Fragment>
        ) : (
          <TableRow key={row.id}>
            <DataTableCells row={row} />
          </TableRow>
        )
      )}
      {rows.length === 0 && empty}
    </>
  );
}

function Spacer({ height, colSpan }: { height: number; colSpan: number }) {
  return (
    <TableRow
      aria-hidden
      style={{ height }}
      className="border-0 transition-none hover:bg-transparent"
    >
      <TableCell colSpan={colSpan} className="p-0" />
    </TableRow>
  );
}
