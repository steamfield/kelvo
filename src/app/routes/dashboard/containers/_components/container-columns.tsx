import type { DockerContainer } from "@core/generated/bindings";
import { useMemo } from "react";
import { columnHelper, gap } from "~/components/data-table";
import {
  containerSortDescFirst,
  isContainerSortColumn,
  isLiveLifecycle,
  lifecycleLabel,
} from "../_lib/presentation";

const helper = columnHelper<DockerContainer>();
function extraColumns(historical: boolean) {
  return {
    io: helper.accessor(
      (row) => {
        const metrics = row.metrics;
        return gap(
          (!historical && !isLiveLifecycle(row)) ||
            metrics.block_read_bps === null ||
            metrics.block_write_bps === null
            ? null
            : metrics.block_read_bps + metrics.block_write_bps
        );
      },
      { id: "io", header: "Block I/O", meta: { align: "right" } }
    ),
    image: helper.accessor("image", {
      header: "Image",
      sortFn: "text",
      meta: { align: "right" },
    }),
    ports: helper.accessor((row) => gap(row.ports.join(", ") || null), {
      id: "ports",
      header: "Ports",
      sortFn: "text",
      meta: { align: "right" },
    }),
    restarts: helper.accessor((row) => gap(row.restart_count), {
      id: "restarts",
      header: "Restarts",
      meta: { align: "right" },
    }),
    // The current wire model has no PID count; its cell is always a gap.
    pids: helper.display({
      id: "pids",
      header: "PIDs",
      meta: { align: "right" },
    }),
  };
}

export const containerRowId = (row: DockerContainer) => row.id;
const byId = (a: DockerContainer, b: DockerContainer) =>
  a.id.localeCompare(b.id);
export const containerTiebreak = byId;

/** One column set drives the shared head and the grouped body. */
export function useContainerColumns(
  optional: readonly string[],
  historical: boolean
) {
  const optionalKey = optional.join(",");
  return useMemo(() => {
    const extra = extraColumns(historical);
    return helper
      .columns([
        helper.display({
          id: "swatch",
          header: () => <span className="sr-only">Chart color</span>,
          meta: { align: "left", className: "w-5 p-0" },
        }),
        helper.accessor("name", {
          header: "Name / project",
          sortFn: "locale",
          meta: {
            align: "left",
            className: "w-[32%] [@container(max-width:480px)]:w-auto",
          },
        }),
        helper.accessor((row) => lifecycleLabel[row.lifecycle], {
          id: "state",
          header: "State",
          sortFn: "text",
          meta: {
            align: "left",
            className: "[@container(max-width:480px)]:hidden",
            cellClassName: "[@container(max-width:480px)]:hidden",
          },
        }),
        helper.accessor(
          (row) =>
            gap(
              historical || isLiveLifecycle(row) ? row.metrics.cpu_pct : null
            ),
          {
            id: "cpu",
            header: historical ? "Peak CPU" : "CPU",
            meta: {
              align: "right",
              className: "[@container(max-width:480px)]:w-[52px]",
            },
          }
        ),
        helper.accessor(
          (row) =>
            gap(
              historical || isLiveLifecycle(row)
                ? row.metrics.working_set_bytes
                : null
            ),
          {
            id: "memory",
            header: historical ? "Peak memory" : "Memory",
            meta: {
              align: "right",
              className: "[@container(max-width:480px)]:w-[92px]",
              cellClassName: "[@container(max-width:480px)]:w-[92px]",
            },
          }
        ),
        ...(!historical
          ? [
              helper.accessor(
                (row) =>
                  gap(
                    (!historical && !isLiveLifecycle(row)) ||
                      row.metrics.network_rx_bps === null ||
                      row.metrics.network_tx_bps === null
                      ? null
                      : row.metrics.network_rx_bps + row.metrics.network_tx_bps
                  ),
                {
                  id: "network",
                  header: "Net",
                  meta: {
                    align: "right",
                    className: "[@container(max-width:719px)]:hidden",
                    cellClassName: "[@container(max-width:719px)]:hidden",
                  },
                }
              ),
            ]
          : []),
        ...optionalKey.split(",").flatMap((id) =>
          Object.entries(extra)
            .filter(([key]) => key === id)
            .map(([, column]) => column)
        ),
        helper.display({
          id: "action",
          header: () => <span className="sr-only">Action</span>,
          meta: { align: "right", className: "w-18 p-0" },
        }),
      ])
      .map((column) => {
        const id =
          column.id ??
          ("accessorKey" in column ? column.accessorKey : undefined);
        return id && isContainerSortColumn(id)
          ? { ...column, sortDescFirst: containerSortDescFirst(id) }
          : column;
      });
  }, [optionalKey, historical]);
}
