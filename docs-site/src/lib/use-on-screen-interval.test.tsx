import { act, renderHook } from "@testing-library/react";
import type { RefObject } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useOnScreenInterval } from "./use-on-screen-interval";

type Entry = { isIntersecting: boolean };
let report: (entries: Entry[]) => void = () => undefined;
let disconnected = false;
class FakeObserver {
  constructor(callback: (entries: Entry[]) => void) {
    report = callback;
  }
  observe() {}
  disconnect() {
    disconnected = true;
  }
}

const target = { current: document.createElement("div") } as RefObject<Element>;

function reducedMotion(reduce: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce,
    media: query,
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
  renderHook(() => useOnScreenInterval(target, tick, 1000));
  vi.advanceTimersByTime(3000);
  expect(tick).not.toHaveBeenCalled();

  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(3000);
  expect(tick).toHaveBeenCalledTimes(3);
});

it("defaults to one tick a second", () => {
  const tick = vi.fn();
  renderHook(() => useOnScreenInterval(target, tick));
  act(() => report([{ isIntersecting: true }]));
  vi.advanceTimersByTime(2000);
  expect(tick).toHaveBeenCalledTimes(2);
});

it("stops when scrolled away and restarts when it returns", () => {
  const tick = vi.fn();
  renderHook(() => useOnScreenInterval(target, tick, 1000));
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
  renderHook(() => useOnScreenInterval(target, tick, 1000));
  act(() => report([{ isIntersecting: true }]));
  const state = vi.spyOn(document, "visibilityState", "get");
  state.mockReturnValue("hidden");
  document.dispatchEvent(new Event("visibilitychange"));
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
  state.mockReturnValue("visible");
  document.dispatchEvent(new Event("visibilitychange"));
  vi.advanceTimersByTime(1000);
  expect(tick).toHaveBeenCalledTimes(1);
});

it("calls the latest tick function without restarting the timer", () => {
  const first = vi.fn();
  const second = vi.fn();
  const { rerender } = renderHook(
    ({ fn }) => useOnScreenInterval(target, fn, 1000),
    { initialProps: { fn: first } }
  );
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
  renderHook(() => useOnScreenInterval(target, tick, 1000));
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
});

it("stops the timer and disconnects on unmount", () => {
  const tick = vi.fn();
  const { unmount } = renderHook(() => useOnScreenInterval(target, tick, 1000));
  act(() => report([{ isIntersecting: true }]));
  unmount();
  vi.advanceTimersByTime(5000);
  expect(tick).not.toHaveBeenCalled();
  expect(disconnected).toBe(true);
});
