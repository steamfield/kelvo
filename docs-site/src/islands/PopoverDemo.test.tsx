import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { PopoverDemo } from "./PopoverDemo";

function screenWidth(wide: boolean) {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        matches: wide,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList
  );
}

/** Posts `data` to the page as if it came from `source`. */
function post(data: unknown, source: Window | null) {
  act(() => {
    window.dispatchEvent(
      new MessageEvent("message", {
        data,
        origin: window.location.origin,
        source,
      })
    );
  });
}

afterEach(() => vi.restoreAllMocks());

it("loads the real popover straight away on wide screens", () => {
  screenWidth(true);
  render(<PopoverDemo />);
  expect(screen.getByTitle("StatGlass popover, live demo")).toHaveAttribute(
    "src",
    "/demo/index.html?window=popover&scenario=popover-compact"
  );
});

it("shows the CPU card on phones and loads the popover only when asked", () => {
  screenWidth(false);
  render(<PopoverDemo />);
  expect(screen.queryByTitle("StatGlass popover, live demo")).toBeNull();
  expect(screen.getByRole("region", { name: "CPU" })).toBeInTheDocument();

  fireEvent.click(
    screen.getByRole("button", { name: "Load the live popover" })
  );

  expect(screen.getByTitle("StatGlass popover, live demo")).toBeInTheDocument();
});

it("keeps the collapsed popover's height when cards expand", () => {
  screenWidth(true);
  render(<PopoverDemo />);
  const frame = screen.getByTitle<HTMLIFrameElement>(
    "StatGlass popover, live demo"
  );

  // A size from another window is ignored, and doesn't use up the first.
  post({ type: "statglass-demo:size", height: 300 }, window);
  expect(frame.parentElement).toHaveStyle({ height: "433px" });

  post({ type: "statglass-demo:size", height: 428 }, frame.contentWindow);
  expect(frame.parentElement).toHaveStyle({ height: "428px" });

  // Expanding a card: the popover scrolls inside the same frame.
  post({ type: "statglass-demo:size", height: 900 }, frame.contentWindow);
  expect(frame.parentElement).toHaveStyle({ height: "428px" });
});

it("opens the dashboard demo in a new tab when a card is clicked", () => {
  screenWidth(true);
  const open = vi.spyOn(window, "open").mockReturnValue(null);
  render(<PopoverDemo />);
  const frame = screen.getByTitle<HTMLIFrameElement>(
    "StatGlass popover, live demo"
  );

  post(
    { type: "statglass-demo:open-dashboard", route: "/dashboard/cpu" },
    frame.contentWindow
  );

  expect(open).toHaveBeenCalledWith(
    "/demo/index.html?window=dashboard&route=%2Fdashboard%2Fcpu",
    "_blank",
    "noopener"
  );
});
