import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ArticleDemo } from "./ArticleDemo";

function viewport(wide: boolean) {
  vi.spyOn(window, "matchMedia").mockImplementation((query) => ({
    matches: query.includes("min-width") && wide,
    media: query,
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent: () => true,
  }));
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(
        private callback: (entries: { isIntersecting: boolean }[]) => void
      ) {}
      observe() {
        this.callback([{ isIntersecting: true }]);
      }
      unobserve() {}
      disconnect() {}
    }
  );
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("offers a simulated GPU card and dashboard link without loading an iframe on narrow screens", () => {
  viewport(false);
  render(
    <ArticleDemo
      route="/dashboard/gpu"
      title="GPU dashboard"
      caption="Try the GPU page."
      fallback="gpu"
    />
  );
  expect(screen.getByText("Simulated reading")).toBeInTheDocument();
  expect(screen.getByText("Frequency")).toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: /Open the dashboard/ })
  ).toHaveAttribute(
    "href",
    "/demo/index.html?window=dashboard&route=%2Fdashboard%2Fgpu"
  );
  expect(screen.queryByTitle("GPU dashboard")).not.toBeInTheDocument();
});

it("loads a native dashboard with the requested title and parameters when a wide viewport is near it", () => {
  viewport(true);
  render(
    <ArticleDemo
      route="/dashboard/processes?columns=gpu"
      params={{ scenario: "multi-host" }}
      title="GPU processes"
      caption="Try the process table."
    />
  );
  const frame = screen.getByTitle("GPU processes");
  expect(frame).toHaveAttribute(
    "src",
    "/demo/index.html?window=dashboard&scenario=multi-host&route=%2Fdashboard%2Fprocesses%3Fcolumns%3Dgpu"
  );
  expect(frame).toHaveClass("ph-no-capture");
  expect(screen.getByText("Try the process table.")).toBeInTheDocument();
});
