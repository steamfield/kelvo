import type { MockEmbed, MockTransport } from "./contracts";

/**
 * Messages between a mock-transport build embedded in an iframe (docs-site's
 * live demo) and the page around it. Same-origin only.
 *
 * Out: `open-dashboard` when the app asks for the dashboard window (a card,
 * the "Open dashboard" button), `size` with the popover's content height
 * (the native panel sizes itself to it; an iframe can't).
 * In: `hide` / `show`, which the mock treats as the window being hidden and
 * shown again, as Rust does: no frames while hidden, then the missed span.
 */
export type DemoOutMessage =
  | { type: "statglass-demo:open-dashboard"; route: string | null }
  | { type: "statglass-demo:size"; height: number };
export type DemoInMessage =
  | { type: "statglass-demo:hide" }
  | { type: "statglass-demo:show" };

/** The parts of `window` the bridge uses, so tests can pass a fake. */
export interface EmbedWindow {
  readonly parent: {
    postMessage(message: unknown, targetOrigin: string): void;
  };
  readonly location: { readonly origin: string };
  addEventListener(type: "message", listener: (e: MessageEvent) => void): void;
  removeEventListener(
    type: "message",
    listener: (e: MessageEvent) => void
  ): void;
}

export interface EmbedBridge {
  /** Pass as `MockTransportOptions.embed`. */
  readonly embed: MockEmbed;
  /** Start applying the parent's messages to `transport`. Returns a stop. */
  attach(transport: Pick<MockTransport, "setWindowVisible">): () => void;
}

export function createEmbedBridge(win: EmbedWindow): EmbedBridge {
  const origin = win.location.origin;
  // An opaque origin (sandboxed iframe, file://) has no targetOrigin to post
  // to, and postMessage would throw; the bridge then does nothing.
  const post = (message: DemoOutMessage) => {
    if (origin !== "null") win.parent.postMessage(message, origin);
  };
  return {
    embed: {
      openDashboard: (route) =>
        post({ type: "statglass-demo:open-dashboard", route }),
      popoverContentHeight: (height) =>
        post({ type: "statglass-demo:size", height }),
    },
    attach(transport) {
      const onMessage = (e: MessageEvent) => {
        if (e.origin !== origin || e.source !== win.parent) return;
        const type = (e.data as Partial<DemoInMessage> | null)?.type;
        if (type === "statglass-demo:hide") transport.setWindowVisible(false);
        else if (type === "statglass-demo:show")
          transport.setWindowVisible(true);
      };
      win.addEventListener("message", onMessage);
      return () => win.removeEventListener("message", onMessage);
    },
  };
}
