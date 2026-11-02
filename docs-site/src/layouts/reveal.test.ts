import { initReveal } from "@steamfield/ui/reveal";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

// BaseLayout calls initReveal() from a module script; these tests pin what
// the landing page relies on from it.
const layout = readFileSync(
  join(process.cwd(), "src/layouts/BaseLayout.astro"),
  "utf8"
);

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
  disconnect() {}
}

function run(reduce: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  return initReveal();
}

let dispose: () => void = () => {};
beforeEach(() => {
  observed.length = 0;
  unobserved.length = 0;
  options = undefined;
  delete document.documentElement.dataset.motionReady;
  document.body.innerHTML = `<section data-reveal id="a"></section><section data-reveal id="b"></section><p id="c"></p>`;
});
afterEach(() => {
  dispose();
  dispose = () => {};
  vi.unstubAllGlobals();
});

it("is started by the layout's script", () => {
  expect(layout).toContain(
    'import { initReveal } from "@steamfield/ui/reveal"'
  );
  expect(layout).toMatch(/^\s*initReveal\(\);$/m);
});

it("marks the page motion-ready and watches every [data-reveal] container", () => {
  dispose = run(false);
  // The attribute's value is "true" in the library; the CSS matches presence.
  expect(document.documentElement).toHaveAttribute("data-motion-ready");
  expect(observed.map((el) => el.id)).toEqual(["a", "b"]);
  expect(options).toEqual({ rootMargin: "0px 0px -12% 0px", threshold: 0 });
});

it("reveals a container once, the first time it enters the viewport", () => {
  dispose = run(false);
  const [a, b] = observed as [Element, Element];
  report([
    { target: a, isIntersecting: false },
    { target: b, isIntersecting: true },
  ]);
  expect(a.hasAttribute("data-revealed")).toBe(false);
  expect(b).toHaveAttribute("data-revealed");
  expect(unobserved).toEqual([b]);
});

it("leaves everything visible and untouched under reduced motion", () => {
  dispose = run(true);
  expect(document.documentElement).not.toHaveAttribute("data-motion-ready");
  expect(observed).toEqual([]);
  expect(document.querySelector("[data-revealed]")).toBeNull();
});
