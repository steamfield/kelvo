import { render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import { ArticleDemo } from "./ArticleDemo";
import { DashboardDemo } from "./DashboardDemo";
import { PopoverDemo } from "./PopoverDemo";

// The demos run the real app, which shows typed filter text as plain table text.
// Session replays (after Accept) must not record them: PostHog's recorder skips any
// element with the ph-no-capture class.

function wideScreen() {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        matches: true,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList
  );
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("keeps the popover demo out of session replays", () => {
  wideScreen();
  render(<PopoverDemo />);
  expect(screen.getByTitle("StatGlass popover, live demo")).toHaveClass(
    "ph-no-capture"
  );
});

it("keeps the dashboard demo out of session replays", () => {
  wideScreen();
  // On screen at once, so the dashboard frame mounts.
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(
        private readonly callback: (
          entries: { isIntersecting: boolean }[]
        ) => void
      ) {}
      observe() {
        this.callback([{ isIntersecting: true }]);
      }
      disconnect() {}
    }
  );
  render(<DashboardDemo route="/" />);
  expect(screen.getByTitle("StatGlass dashboard, live demo")).toHaveClass(
    "ph-no-capture"
  );
});

it("keeps the article dashboard out of session replays", () => {
  wideScreen();
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(
        private callback: (entries: { isIntersecting: boolean }[]) => void
      ) {}
      observe() {
        this.callback([{ isIntersecting: true }]);
      }
      disconnect() {}
    }
  );
  render(
    <ArticleDemo
      route="/dashboard/gpu"
      title="Article GPU"
      caption="GPU example"
    />
  );
  expect(screen.getByTitle("Article GPU")).toHaveClass("ph-no-capture");
});
