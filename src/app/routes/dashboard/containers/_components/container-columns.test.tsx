import { act, renderHook, waitFor } from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import { useEffect } from "react";
import { useHost } from "~/stores/host-store";
import { useContainerColumns } from "./container-columns";

it("keeps column definitions stable across fresh optional arrays on pushed frames", async () => {
  const observed: {
    now: number | null;
    columns: ReturnType<typeof useContainerColumns>;
  }[] = [];
  function Probe() {
    const now = useHost((s) => s.lastTsMs);
    const columns = useContainerColumns(["image", "ports"], false);
    useEffect(() => {
      observed.push({ now, columns });
    }, [now, columns]);
    return null;
  }
  const result = renderWithProviders(<Probe />);
  await waitFor(() => expect(result.transport.subscriberCount()).toBe(1));
  const initial = observed.at(-1)?.columns;
  await act(() => result.transport.tick());
  expect(observed.length).toBeGreaterThan(1);
  expect(observed.at(-1)?.columns).toBe(initial);
});

it("rebuilds columns when optional columns or historical mode change", () => {
  const { result, rerender } = renderHook(
    ({ optional, historical }) => useContainerColumns(optional, historical),
    { initialProps: { optional: ["image"], historical: false } }
  );
  const initial = result.current;
  rerender({ optional: ["ports"], historical: false });
  expect(result.current).not.toBe(initial);
  expect(result.current.some((c) => c.id === "ports")).toBe(true);
  rerender({ optional: ["ports"], historical: true });
  expect(result.current.some((c) => c.id === "network")).toBe(false);
});
