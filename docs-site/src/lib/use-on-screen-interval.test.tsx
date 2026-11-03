import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useOnScreenInterval } from "./use-on-screen-interval";

type Entry = { isIntersecting: boolean };
let report: (entries: Entry[]) => void = () => undefined;
let disconnected = false;
// Real observers report each entry with its target; the gate ignores entries for elements it no
// longer watches, so the fake reports for the element it last observed.
class FakeObserver {
  target: Element | undefined;
  constructor(
    callback: (entries: (Entry & { target?: Element | undefined })[]) => void
  ) {
    report = (entries) =>
      callback(entries.map((entry) => ({ target: this.target, ...entry })));
  }
  observe(element: Element) {
    this.target = element;
  }
  disconnect() {
    disconnected = true;
  }
}

/** Renders the hook and attaches its ref to an element, as a component's JSX does. */
function mount(tick: () => void, intervalMs?: number) {
  const hook = renderHook(() => useOnScreenInterval(tick, intervalMs));
  act(() => {
    hook.result.current(document.createElement("div"));
  });
  return hook;
}

function reducedMotion(reduce: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

beforeEach(() => {
  vi.useFakeTimers();
  disconnected = false;
  vi.stubGlobal("IntersectionObserver", FakeObserver);
  reducedMotion(false);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it("ticks every interval while on screen, and not before it is seen", () => {
  const tick = vi.fn();
  mount(tick, 1000);
  vi.advanceTimersByTime(3000);
  expect(tick).not.toHaveBeenCalled();

  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(3000);
  expect(tick).toHaveBeenCalledTimes(3);
});

it("defaults to one tick a second", () => {
  const tick = vi.fn();
  mount(tick);
  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(2000);
  expect(tick).toHaveBeenCalledTimes(2);
});

it("stops when scrolled away and restarts when it returns", () => {
  const tick = vi.fn();
  mount(tick, 1000);
  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(1000);
  act(() => report([{ isIntersecting: false }]));
  vi.advanceTimersByTime(5000);
  expect(tick).toHaveBeenCalledTimes(1);
  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(1000);
  expect(tick).toHaveBeenCalledTimes(2);
});

it("pauses while the tab is hidden", () => {
  const tick = vi.fn();
  mount(tick, 1000);
  act(() => report([{ isIntersecting: true }]));
  const state = vi.spyOn(document, "visibilityState", "get");
  state.mockReturnValue("hidden");
  act(() => {
    document.dispatchEvent(new Event("visibilitychange"));
  });
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
  state.mockReturnValue("visible");
  act(() => {
    document.dispatchEvent(new Event("visibilitychange"));
  });
  vi.advanceTimersByTime(1000);
  expect(tick).toHaveBeenCalledTimes(1);
});

it("calls the latest tick function without restarting the timer", () => {
  const first = vi.fn();
  const second = vi.fn();
  const { result, rerender } = renderHook(
    ({ fn }) => useOnScreenInterval(fn, 1000),
    { initialProps: { fn: first } }
  );
  act(() => {
    result.current(document.createElement("div"));
  });
  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(500);
  rerender({ fn: second });
  vi.advanceTimersByTime(500);
  expect(first).not.toHaveBeenCalled();
  expect(second).toHaveBeenCalledTimes(1);
});

it("never ticks under reduced motion, so the demo holds its first frame", () => {
  reducedMotion(true);
  const tick = vi.fn();
  mount(tick, 1000);
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
});

it("stops the timer and disconnects on unmount", () => {
  const tick = vi.fn();
  const { unmount } = mount(tick, 1000);
  act(() => report([{ isIntersecting: true }]));
  unmount();
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
  expect(disconnected).toBe(true);
});
