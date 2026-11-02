import { afterEach, expect, it, vi } from "vitest";
import { spyToc } from "./docs-enhance";

type Callback = (
  entries: Array<{ target: Element; isIntersecting: boolean }>
) => void;

// The browser decides what is in view; the test plays that part.
let report: Callback = () => undefined;
class FakeObserver {
  constructor(callback: Callback) {
    report = callback;
  }
  observe() {}
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

function page() {
  document.body.innerHTML = `
    <nav data-toc>
      <a data-toc-link href="#install">Install</a>
      <a data-toc-link href="#build-it">Build it</a>
    </nav>
    <h2 id="install">Install</h2>
    <h2 id="build-it">Build it</h2>`;
  const link = (name: string) =>
    document
      .querySelector(`[data-toc-link][href="#${name}"]`)
      ?.getAttribute("aria-current");
  return {
    link,
    heading: (id: string) => document.getElementById(id) as Element,
  };
}

it("marks the first section in view and moves the mark as reading continues", () => {
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  const { link, heading } = page();
  spyToc(document);
  report([{ target: heading("install"), isIntersecting: true }]);
  expect([link("install"), link("build-it")]).toEqual(["location", null]);
  report([
    { target: heading("install"), isIntersecting: false },
    { target: heading("build-it"), isIntersecting: true },
  ]);
  expect([link("install"), link("build-it")]).toEqual([null, "location"]);
});

it("keeps the last mark when every heading scrolls out of view", () => {
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  const { link, heading } = page();
  spyToc(document);
  report([{ target: heading("build-it"), isIntersecting: true }]);
  report([{ target: heading("build-it"), isIntersecting: false }]);
  expect(link("build-it")).toBe("location");
});

it("leaves the links unmarked without IntersectionObserver", () => {
  vi.stubGlobal("IntersectionObserver", undefined);
  Reflect.deleteProperty(window, "IntersectionObserver");
  const { link } = page();
  spyToc(document);
  expect([link("install"), link("build-it")]).toEqual([null, null]);
});
