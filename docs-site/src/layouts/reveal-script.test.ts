import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

// The reveal script is inline in BaseLayout.astro, so the test pulls it out of
// the source and runs it the way the browser would.
const layout = readFileSync(
  join(process.cwd(), "src/layouts/BaseLayout.astro"),
  "utf8"
);
const script = /<script is:inline>\s*\/\/ Scroll reveals[\s\S]*?<\/script>/
  .exec(layout)?.[0]
  .replace(/^<script is:inline>/, "")
  .replace(/<\/script>$/, "");

type Entry = { target: Element; isIntersecting: boolean };
let report: (entries: Entry[]) => void = () => undefined;
let options: IntersectionObserverInit | undefined;
const observed: Element[] = [];
const unobserved: Element[] = [];
class FakeObserver {
  constructor(
    callback: (entries: Entry[]) => void,
    init?: IntersectionObserverInit
  ) {
    report = callback;
    options = init;
  }
  observe(el: Element) {
    observed.push(el);
  }
  unobserve(el: Element) {
    unobserved.push(el);
  }
}

function run(reduce: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce,
    media: query,
  }));
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  new Function(script ?? "")();
  document.dispatchEvent(new Event("DOMContentLoaded"));
}

beforeEach(() => {
  observed.length = 0;
  unobserved.length = 0;
  options = undefined;
  delete document.documentElement.dataset.motionReady;
  document.body.innerHTML = `<section data-reveal id="a"></section><section data-reveal id="b"></section><p id="c"></p>`;
});
afterEach(() => vi.unstubAllGlobals());

it("found the inline script in the layout", () => {
  expect(script).toContain("data-revealed");
});

it("marks the page motion-ready and watches every [data-reveal] container", () => {
  run(false);
  expect(document.documentElement.dataset.motionReady).toBe("");
  expect(observed.map((el) => el.id)).toEqual(["a", "b"]);
  expect(options).toEqual({ rootMargin: "0px 0px -12% 0px" });
});

it("reveals a container once, the first time it enters the viewport", () => {
  run(false);
  const [a, b] = observed as [Element, Element];
  report([
    { target: a, isIntersecting: false },
    { target: b, isIntersecting: true },
  ]);
  expect(a.hasAttribute("data-revealed")).toBe(false);
  expect(b.getAttribute("data-revealed")).toBe("");
  expect(unobserved).toEqual([b]);
});

it("leaves everything visible and untouched under reduced motion", () => {
  run(true);
  expect(document.documentElement.dataset.motionReady).toBeUndefined();
  expect(observed).toEqual([]);
  expect(document.querySelector("[data-revealed]")).toBeNull();
});
