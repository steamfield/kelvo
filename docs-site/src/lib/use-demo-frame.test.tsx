import { act, renderHook } from "@testing-library/react";
import type { RefObject } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { OFFSCREEN_GRACE_MS, useDemoFrame } from "./use-demo-frame";

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

function frame() {
  const contentWindow = { postMessage: vi.fn() };
  const ref = {
    current: { contentWindow } as unknown as HTMLIFrameElement,
  } as RefObject<HTMLIFrameElement | null>;
  return { ref, contentWindow };
}

function visibility(state: "visible" | "hidden") {
  vi.spyOn(document, "visibilityState", "get").mockReturnValue(state);
  document.dispatchEvent(new Event("visibilitychange"));
}

function post(data: unknown, source: unknown, origin = window.location.origin) {
  act(() => {
    window.dispatchEvent(
      new MessageEvent("message", {
        data,
        origin,
        source: source as MessageEventSource,
      })
    );
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  disconnected = false;
  vi.stubGlobal("IntersectionObserver", FakeObserver);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it("hands the frame's messages to the callback, and ignores anyone else's", () => {
  const { ref, contentWindow } = frame();
  const onMessage = vi.fn();
  renderHook(() => useDemoFrame(ref, true, onMessage));

  post({ type: "statglass-demo:size", height: 400 }, contentWindow);
  expect(onMessage).toHaveBeenCalledWith({ type: "size", height: 400 });

  onMessage.mockClear();
  post({ type: "statglass-demo:size", height: 400 }, window); // wrong source
  post(
    { type: "statglass-demo:size", height: 400 },
    contentWindow,
    "https://evil.example"
  ); // wrong origin
  post({ type: "other" }, contentWindow); // not a demo message
  expect(onMessage).not.toHaveBeenCalled();
});

it("does nothing until the frame is mounted", () => {
  const { ref, contentWindow } = frame();
  const onMessage = vi.fn();
  renderHook(() => useDemoFrame(ref, false, onMessage));
  post({ type: "statglass-demo:size", height: 400 }, contentWindow);
  expect(onMessage).not.toHaveBeenCalled();
});

it("shows the demo at once on scrolling back, and hides it only after the grace period", () => {
  const { ref, contentWindow } = frame();
  renderHook(() => useDemoFrame(ref, true, vi.fn()));
  const origin = window.location.origin;

  act(() => report([{ isIntersecting: false }]));
  vi.advanceTimersByTime(OFFSCREEN_GRACE_MS - 1);
  expect(contentWindow.postMessage).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(contentWindow.postMessage).toHaveBeenLastCalledWith(
    { type: "statglass-demo:hide" },
    origin
  );

  act(() => report([{ isIntersecting: true }]));
  expect(contentWindow.postMessage).toHaveBeenLastCalledWith(
    { type: "statglass-demo:show" },
    origin
  );
});

it("cancels a pending hide when the frame comes back on screen", () => {
  const { ref, contentWindow } = frame();
  renderHook(() => useDemoFrame(ref, true, vi.fn()));
  act(() => report([{ isIntersecting: false }]));
  vi.advanceTimersByTime(OFFSCREEN_GRACE_MS - 100);
  act(() => report([{ isIntersecting: true }]));
  contentWindow.postMessage.mockClear();
  vi.advanceTimersByTime(OFFSCREEN_GRACE_MS);
  expect(contentWindow.postMessage).not.toHaveBeenCalled();
});

it("hides the demo while the tab is hidden and shows it again when visible", () => {
  const { ref, contentWindow } = frame();
  renderHook(() => useDemoFrame(ref, true, vi.fn()));
  visibility("hidden");
  expect(contentWindow.postMessage).toHaveBeenLastCalledWith(
    { type: "statglass-demo:hide" },
    window.location.origin
  );
  visibility("visible");
  expect(contentWindow.postMessage).toHaveBeenLastCalledWith(
    { type: "statglass-demo:show" },
    window.location.origin
  );
});

it("stops listening and disconnects on unmount", () => {
  const { ref, contentWindow } = frame();
  const onMessage = vi.fn();
  const { unmount } = renderHook(() => useDemoFrame(ref, true, onMessage));
  unmount();
  expect(disconnected).toBe(true);
  post({ type: "statglass-demo:size", height: 400 }, contentWindow);
  expect(onMessage).not.toHaveBeenCalled();
});
