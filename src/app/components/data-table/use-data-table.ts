import { holdOrder } from "@core/held-order";
import {
  type ColumnDef,
  type OnChangeFn,
  type RowData,
  type SortingState,
  useTable,
} from "@tanstack/react-table";
import { useMemo, useRef, useState } from "react";
import { features, type TableFeatures } from "./features";

interface DataTableOptions<T extends RowData> {
  data: readonly T[];
  columns: readonly ColumnDef<TableFeatures, T>[];
  getRowId: (row: T) => string;
  tiebreak?: (a: T, b: T) => number;
  sorting?: SortingState;
  onSortingChange?: OnChangeFn<SortingState>;
  initialSorting?: SortingState;
  manualSorting?: boolean;
  hold?: { paused: boolean; resetKey: string };
}

export function useDataTable<T extends RowData>({
  data,
  columns,
  getRowId,
  tiebreak,
  sorting,
  onSortingChange,
  initialSorting = [],
  manualSorting = false,
  hold,
}: DataTableOptions<T>) {
  const [internalSorting, setSorting] = useState(initialSorting);
  const activeSorting = sorting ?? internalSorting;
  const compare = useMemo(
    () =>
      tiebreak ??
      ((a: T, b: T) => {
        const aId = getRowId(a);
        const bId = getRowId(b);
        return aId < bId ? -1 : aId > bId ? 1 : 0;
      }),
    [tiebreak, getRowId]
  );
  const hasSorting = activeSorting.length > 0;
  // The index fallback in TanStack stays independent of incoming frame order
  // and sort direction. Manual tables and unsorted tables keep source order.
  const ordered = useMemo(
    () => (manualSorting || !hasSorting ? data : [...data].sort(compare)),
    [data, compare, manualSorting, hasSorting]
  );
  const alignedColumns = useMemo(
    () =>
      columns.map((column) => ({
        sortDescFirst: column.meta?.align !== "left",
        ...column,
      })),
    [columns]
  );
  const table = useTable(
    {
      features,
      data: ordered,
      columns: alignedColumns,
      getRowId,
      state: { sorting: activeSorting },
      initialState: { sorting: initialSorting },
      onSortingChange: onSortingChange ?? setSorting,
      manualSorting,
      defaultColumn: { sortUndefined: "last" },
      enableSorting: true,
      enableMultiSort: false,
      enableSortingRemoval: false,
      sortDescFirst: true,
    },
    (state) => ({ sorting: state.sorting })
  );
  const sorted = table.getRowModel().rows;
  const held = useRef<{ resetKey: string; ids: string[] } | null>(null);
  if (!hold?.paused) held.current = null;
  else if (held.current?.resetKey !== hold.resetKey)
    held.current = {
      resetKey: hold.resetKey,
      ids: sorted.map((row) => row.id),
    };
  const rows = held.current
    ? holdOrder(sorted, held.current.ids, (row) => row.id)
    : sorted;
  return { table, rows };
}
