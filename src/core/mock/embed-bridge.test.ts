import type { LiveMsg } from "@core/generated/bindings";
import { createMockTransport } from "../mock-transport";
import { createEmbedBridge, type EmbedWindow } from "./embed-bridge";
import { MOCK_HOST_ID } from "./fixtures";

const ORIGIN = "https://statglass.example";
const NOW = 1_800_000_000_000;

/** An iframe's `window`: posts land in `posted`; `deliver` plays the parent. */
function fakeWindow() {
  const posted: { message: unknown; targetOrigin: string }[] = [];
  const listeners = new Set<(e: MessageEvent) => void>();
  const parent = {
    postMessage: (message: unknown, targetOrigin: string) =>
      posted.push({ message, targetOrigin }),
  };
  const win: EmbedWindow = {
    parent,
    location: { origin: ORIGIN },
    addEventListener: (_type, l) => listeners.add(l),
    removeEventListener: (_type, l) => listeners.delete(l),
  };
  const deliver = (
    data: unknown,
    from = { origin: ORIGIN, source: parent }
  ) => {
    for (const l of listeners)
      l({
        data,
        origin: from.origin,
        source: from.source,
      } as unknown as MessageEvent);
  };
  return { win, parent, posted, deliver };
}

async function embedded() {
  const w = fakeWindow();
  const bridge = createEmbedBridge(w.win);
  const transport = createMockTransport({
    now: () => NOW,
    windowLabel: "popover",
    embed: bridge.embed,
  });
  const stop = bridge.attach(transport);
  const msgs: LiveMsg[] = [];
  await transport.subscribeLive(MOCK_HOST_ID, (m) => msgs.push(m));
  msgs.length = 0;
  return { ...w, transport, msgs, stop };
}

const frames = (msgs: LiveMsg[]) => msgs.filter((m) => m.kind === "frame");

describe("embed bridge", () => {
  it("tells the parent page when the app asks for the dashboard", async () => {
    const { transport, posted } = await embedded();

    await transport.openDashboard("/dashboard/cpu");

    expect(posted).toEqual([
      {
        message: {
          type: "statglass-demo:open-dashboard",
          route: "/dashboard/cpu",
        },
        targetOrigin: ORIGIN,
      },
    ]);
  });

  it("passes the popover's content height to the parent page", async () => {
    const { transport, posted } = await embedded();

    await transport.reportPopoverContentHeight(512);
    await transport.reportPopoverContentHeight(Number.NaN);

    expect(posted.map((p) => p.message)).toEqual([
      { type: "statglass-demo:size", height: 512 },
    ]);
  });

  it("stops frames while the parent hides the demo and resumes on show", async () => {
    const { transport, msgs, deliver } = await embedded();

    deliver({ type: "statglass-demo:hide" });
    transport.tick();
    expect(frames(msgs)).toHaveLength(0);

    deliver({ type: "statglass-demo:show" });
    transport.tick();
    await vi.waitFor(() => expect(frames(msgs).length).toBeGreaterThan(0));
  });

  it("posts nothing from an opaque origin, where postMessage would throw", () => {
    const w = fakeWindow();
    const parent = {
      postMessage: () => {
        throw new SyntaxError("Invalid target origin 'null'");
      },
    };
    const bridge = createEmbedBridge({
      ...w.win,
      parent,
      location: { origin: "null" },
    });

    expect(() => bridge.embed.openDashboard("/dashboard/cpu")).not.toThrow();
    expect(() => bridge.embed.popoverContentHeight(512)).not.toThrow();
  });

  it("ignores messages from other origins or windows, and after stop", async () => {
    const { transport, msgs, deliver, stop } = await embedded();

    deliver(
      { type: "statglass-demo:hide" },
      { origin: "https://evil.example", source: {} as never }
    );
    deliver(
      { type: "statglass-demo:hide" },
      { origin: ORIGIN, source: {} as never }
    );
    transport.tick();
    await vi.waitFor(() => expect(frames(msgs)).toHaveLength(1));

    stop();
    deliver({ type: "statglass-demo:hide" });
    transport.tick();
    await vi.waitFor(() => expect(frames(msgs)).toHaveLength(2));
  });
});
