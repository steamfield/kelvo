import {
  type CellData,
  type TableFeatures as CoreTableFeatures,
  constructSortFn,
  createColumnHelper,
  createSortedRowModel,
  type RowData,
  rowSortingFeature,
  sortFn_alphanumeric,
  sortFn_text,
  tableFeatures,
} from "@tanstack/react-table";

declare module "@tanstack/table-core" {
  interface ColumnMeta<
    in out TFeatures extends CoreTableFeatures,
    in out TData extends RowData,
    TValue extends CellData = CellData,
  > {
    align: "left" | "right";
    className?: string;
    cellClassName?: string;
    ranked?: boolean;
  }
}

export const features = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns: {
    alphanumeric: sortFn_alphanumeric,
    text: sortFn_text,
    locale: constructSortFn({ sort: new Intl.Collator().compare }),
  },
});

export type TableFeatures = typeof features;
export const columnHelper = <T extends RowData>() =>
  createColumnHelper<TableFeatures, T>();
export const gap = <T>(value: T | null): T | undefined => value ?? undefined;
