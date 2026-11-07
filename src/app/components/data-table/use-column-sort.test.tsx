import { act, renderHook } from "@testing-library/react";
import { useState } from "react";
import { type ColumnSort, useColumnSort } from ".";

const ids = ["name", "value"] as const;
type Sort = ColumnSort<(typeof ids)[number]>;

describe("useColumnSort", () => {
  it("memoizes sorting by column and direction, even with a fresh sort object", () => {
    const { result, rerender } = renderHook(
      ({ sort }: { sort: Sort }) => useColumnSort(sort, () => {}, ids),
      { initialProps: { sort: { by: "value", dir: "desc" } } }
    );
    const initial = result.current.sorting;
    rerender({ sort: { by: "value", dir: "desc" } });
    expect(result.current.sorting).toBe(initial);
    rerender({ sort: { by: "value", dir: "asc" } });
    expect(result.current.sorting).not.toBe(initial);
    expect(result.current.sorting).toEqual([{ id: "value", desc: false }]);
    rerender({ sort: { by: "name", dir: "asc" } });
    expect(result.current.sorting).toEqual([{ id: "name", desc: false }]);
  });

  it("maps updater flips and direct updates back to the owner's sort", () => {
    const { result } = renderHook(() => {
      const [sort, setSort] = useState<Sort>({ by: "value", dir: "desc" });
      return { sort, ...useColumnSort(sort, setSort, ids) };
    });
    act(() =>
      result.current.onSortingChange((current) =>
        current.map((entry) => ({ ...entry, desc: !entry.desc }))
      )
    );
    expect(result.current.sort).toEqual({ by: "value", dir: "asc" });
    act(() => result.current.onSortingChange([{ id: "name", desc: true }]));
    expect(result.current.sort).toEqual({ by: "name", dir: "desc" });
  });

  it("ignores unknown, removed and empty sort updates", () => {
    const initialProps: { allowed: readonly (typeof ids)[number][] } = {
      allowed: ids,
    };
    const { result, rerender } = renderHook(
      ({ allowed }: { allowed: readonly (typeof ids)[number][] }) => {
        const [sort, setSort] = useState<Sort>({ by: "value", dir: "desc" });
        return { sort, ...useColumnSort(sort, setSort, allowed) };
      },
      { initialProps }
    );
    const initial = result.current.sort;
    act(() => result.current.onSortingChange([{ id: "unknown", desc: false }]));
    expect(result.current.sort).toBe(initial);
    act(() => result.current.onSortingChange(() => []));
    expect(result.current.sort).toBe(initial);
    rerender({ allowed: ["value"] });
    act(() => result.current.onSortingChange([{ id: "name", desc: false }]));
    expect(result.current.sort).toBe(initial);
  });
});
