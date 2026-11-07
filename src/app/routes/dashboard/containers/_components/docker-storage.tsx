import type { DockerStorageResource } from "@core/generated/bindings";
import { dockerKeys } from "@core/query-keys";
import { unwrap } from "@core/transport";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { constructSortFn } from "@tanstack/react-table";
import { memo, useMemo, useState } from "react";
import {
  columnHelper,
  gap,
  useColumnSort,
  useDataTable,
} from "~/components/data-table";
import { Button } from "~/components/ui/button";
import { useTransport } from "~/lib/transport-context";
import {
  type StorageSort,
  storageResourceKey,
  storageRows,
} from "../_lib/storage-presentation";
import type { CleanupFlow } from "./docker-cleanup-flow";
import { StorageFilters } from "./storage-filters";
import { StorageInventorySummary } from "./storage-inventory-summary";
import { StorageResources } from "./storage-resources";

const SORT_IDS = ["size", "name", "references", "age", "eligibility"] as const;

const nameSort = constructSortFn({ sort: new Intl.Collator().compare });
const helper = columnHelper<DockerStorageResource>();
const COLUMNS = helper.columns([
  helper.accessor("name", {
    header: "Resource / type",
    sortFn: nameSort,
    meta: { align: "left" },
  }),
  helper.accessor((row) => gap(row.size_bytes), {
    id: "size",
    header: "Size",
    meta: { align: "right", className: "text-left [&_button]:text-left" },
  }),
  helper.accessor((row) => row.references.length, {
    id: "references",
    header: "References",
    meta: { align: "right", className: "text-left [&_button]:text-left" },
  }),
  helper.accessor((row) => gap(row.age_ms), {
    id: "age",
    header: "Age evidence",
    meta: { align: "right", className: "text-left [&_button]:text-left" },
  }),
  helper.accessor(
    (row) =>
      row.can_delete ? "Available for review" : (row.reason ?? "Not eligible"),
    {
      id: "eligibility",
      header: "Eligibility",
      sortFn: "text",
      meta: { align: "left" },
    }
  ),
]);
export const DockerStorage = memo(function DockerStorage({
  engine,
  generation,
  connected,
  flow,
}: {
  engine: string;
  generation: number;
  connected: boolean;
  flow: CleanupFlow;
}) {
  const transport = useTransport();
  const client = useQueryClient();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [age, setAge] = useState("7");
  const [sort, setSort] = useState<StorageSort>({ by: "size", dir: "desc" });
  const [grouped, setGrouped] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [page, setPage] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const storage = useQuery({
    queryKey: dockerKeys.storage(engine, generation),
    enabled: Boolean(engine) && connected,
    queryFn: () => unwrap(transport.dockerStorageScan(false)),
    staleTime: 300_000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: false,
  });
  const snapshot = storage.data?.snapshot;
  const filtered = useMemo(
    () => storageRows(snapshot?.resources ?? [], query, category, age),
    [snapshot, query, category, age]
  );
  const { sorting, onSortingChange } = useColumnSort(sort, setSort, SORT_IDS);
  const model = useDataTable({
    data: filtered,
    columns: COLUMNS,
    getRowId: storageResourceKey,
    sorting,
    onSortingChange,
  });
  const rows = model.rows.map((row) => row.original);
  const chosen =
    snapshot?.resources.filter(
      (row) => selected.has(storageResourceKey(row)) && row.can_delete
    ) ?? [];
  const currentPage = Math.min(
    page,
    Math.max(0, Math.ceil(rows.length / 32) - 1)
  );
  const pageRows = rows.slice(currentPage * 32, (currentPage + 1) * 32);
  const groups = grouped
    ? [...new Set(pageRows.map((row) => row.project ?? "No Compose project"))]
    : [""];
  const refresh = async () => {
    setRefreshing(true);
    setRefreshError(null);
    try {
      client.setQueryData(
        dockerKeys.storage(engine, generation),
        await unwrap(transport.dockerStorageScan(true))
      );
    } catch (e) {
      setRefreshError(String(e));
    } finally {
      setRefreshing(false);
    }
  };
  const error = refreshError ?? storage.data?.error ?? storage.error?.message;
  return (
    <section aria-label="Docker storage" className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="font-[590] text-[16px]">Docker-managed storage</h2>
          <p className="mt-1 text-[11px] text-muted-foreground">
            {snapshot
              ? `Measured ${new Date(snapshot.observed_ms).toLocaleString()}`
              : "Scan images, writable layers, volumes and daemon build cache on demand."}
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          disabled={!connected || storage.isFetching || refreshing || flow.busy}
          onClick={() => void refresh()}
        >
          {storage.isFetching || refreshing ? "Scanning…" : "Refresh storage"}
        </Button>
      </div>
      {!connected && (
        <p role="status" className="text-[12px] text-muted-foreground">
          Storage scanning requires a connected engine. Resume monitoring if it
          is paused.
        </p>
      )}
      {error && (
        <p role="alert" className="text-[12px] text-destructive">
          {error}
          {snapshot ? " The previous measurement is retained." : ""}
        </p>
      )}
      {storage.isPending && connected && (
        <p role="status" className="text-[12px] text-muted-foreground">
          Reading Docker storage…
        </p>
      )}
      {snapshot && (
        <>
          <StorageInventorySummary snapshot={snapshot} />
          <StorageFilters
            query={query}
            setQuery={setQuery}
            category={category}
            setCategory={setCategory}
            age={age}
            setAge={setAge}
            grouped={grouped}
            setGrouped={setGrouped}
            setPage={setPage}
          />
          <p className="text-[11px] text-muted-foreground">
            Age filters use stopped time for containers and last use for cache.
            Images and volumes have no reliable unused age.
          </p>
          <StorageResources
            rows={rows}
            chosen={chosen}
            error={error}
            connected={connected}
            snapshot={snapshot}
            flow={flow}
            refreshing={refreshing}
            fetching={storage.isFetching}
            groups={groups}
            grouped={grouped}
            pageRows={pageRows}
            selected={selected}
            setSelected={setSelected}
            table={model.table}
            currentPage={currentPage}
            setPage={setPage}
          />
        </>
      )}
    </section>
  );
});
