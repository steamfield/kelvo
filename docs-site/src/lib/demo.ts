/**
 * The live demo: the real app, built on its mock transport into public/demo
 * (scripts/build-demo.mjs), shown in an iframe. Message shapes mirror the
 * app's embed bridge (../src/core/mock/embed-bridge.ts); the site keeps its
 * own copy because it may import the app only through @statglass/*.
 */

/** The popover panel's native size (src-tauri/src/popover/mod.rs). */
export const POPOVER_WIDTH = 360;
export const POPOVER_MAX_HEIGHT = 680;
/**
 * The demo popover's height with every card collapsed, as its first size
 * message reports it. Used until that message arrives, so the hero doesn't
 * jump when the demo loads.
 */
export const POPOVER_COLLAPSED_HEIGHT = 433;
/** The dashboard window's minimum width (src-tauri/src/windows.rs). */
export const DASHBOARD_MIN_WIDTH = 1024;
export const DASHBOARD_HEIGHT = 760;

/** The popover with every card collapsed, so all of them fit the frame. */
export function popoverDemoUrl(): string {
  return "/demo/index.html?window=popover&scenario=popover-compact";
}

export const DEMO_PARAM_KEYS = [
  "scenario",
  "space_scan",
  "host",
  "chart_window",
] as const;
export type DemoParams = Partial<
  Record<(typeof DEMO_PARAM_KEYS)[number], string>
>;
export const KNOWN_SCENARIOS = [
  "default",
  "linux-host",
  "multi-host",
  "docker-11",
  "vpn",
  "no-battery",
] as const;

/** Outer parameters belong to the demo document; route parameters stay encoded inside route. */
export function dashboardDemoUrl(
  route?: string | null,
  params: DemoParams = {}
): string {
  let url = "/demo/index.html?window=dashboard";
  for (const key of DEMO_PARAM_KEYS) {
    if (params[key] !== undefined)
      url += `&${key}=${encodeURIComponent(params[key])}`;
  }
  return route?.startsWith("/dashboard/")
    ? `${url}&route=${encodeURIComponent(route)}`
    : url;
}

export type DemoMessage =
  | { type: "size"; height: number }
  | { type: "open-dashboard"; route: string | null };

/** Reads a message the demo posted, or null for anything else. */
export function readDemoMessage(data: unknown): DemoMessage | null {
  if (typeof data !== "object" || data === null) return null;
  const message = data as Record<string, unknown>;
  if (message.type === "statglass-demo:size") {
    const height = message.height;
    return typeof height === "number" && Number.isFinite(height) && height > 0
      ? { type: "size", height }
      : null;
  }
  if (message.type === "statglass-demo:open-dashboard") {
    const route = message.route;
    return {
      type: "open-dashboard",
      route: typeof route === "string" ? route : null,
    };
  }
  return null;
}

/** Tells the demo whether it is on screen, so it only ticks while seen. */
export function setDemoVisible(frame: HTMLIFrameElement, visible: boolean) {
  frame.contentWindow?.postMessage(
    { type: visible ? "statglass-demo:show" : "statglass-demo:hide" },
    window.location.origin
  );
}

/**
 * Whether the demo document has rendered in the page's theme. Before that it
 * is a blank (white) canvas: the app applies its theme class from React,
 * after its transport starts.
 */
export function demoPainted(doc: Document, dark: boolean): boolean {
  const root = doc.getElementById("root");
  return (
    root !== null &&
    root.childElementCount > 0 &&
    doc.documentElement.classList.contains("dark") === dark
  );
}
