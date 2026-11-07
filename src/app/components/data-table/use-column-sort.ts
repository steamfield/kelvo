import type { OnChangeFn, SortingState } from "@tanstack/react-table";
import { useCallback, useMemo } from "react";

export type SortDir = "asc" | "desc";

export interface ColumnSort<K extends string> {
  by: K;
  dir: SortDir;
}

/** Translate an owner's column sort at the table boundary. */
export function useColumnSort<K extends string>(
  { by, dir }: ColumnSort<K>,
  onSort: (sort: ColumnSort<K>) => void,
  ids: readonly K[]
) {
  const sorting = useMemo<SortingState>(
    () => [{ id: by, desc: dir === "desc" }],
    [by, dir]
  );
  const onSortingChange = useCallback<OnChangeFn<SortingState>>(
    (updater) => {
      const next = (
        typeof updater === "function" ? updater(sorting) : updater
      )[0];
      const column = ids.find((id) => id === next?.id);
      if (next && column !== undefined)
        onSort({ by: column, dir: next.desc ? "desc" : "asc" });
    },
    [sorting, onSort, ids]
  );
  return { sorting, onSortingChange };
}
