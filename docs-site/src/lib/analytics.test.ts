import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { flushQueue } from "./analytics";
import { EVENTS } from "./events";

const sdk = vi.hoisted(() => ({ init: vi.fn() }));
vi.mock("posthog-js", () => ({ default: sdk }));

/** A stand-in for the PostHog client that records what reached it. */
function fakeClient(): {
  client: Parameters<typeof flushQueue>[0];
  sent: string[];
} {
  const sent: string[] = [];
  const client = {
    capture: (event: string, props?: Record<string, unknown> | null) => {
      sent.push(props ? `${event}:${JSON.stringify(props)}` : event);
      return undefined;
    },
  };
  return { client, sent };
}

async function loadModule() {
  vi.resetModules();
  return import("./analytics");
}

beforeEach(() => {
  sdk.init.mockReset();
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe("without a key", () => {
  it("never loads the SDK and drops calls silently", async () => {
    const warn = vi.spyOn(console, "warn");
    const analytics = await loadModule();

    await analytics.initAnalytics();
    analytics.track(EVENTS.downloadClicked);

    const { client, sent } = fakeClient();
    analytics.flushQueue(client);

    expect(sdk.init).not.toHaveBeenCalled();
    expect(sent).toEqual([]);
    expect(warn).not.toHaveBeenCalled();
  });
});

describe("with a key", () => {
  beforeEach(() => {
    vi.stubEnv("PUBLIC_POSTHOG_KEY", "phc_test");
  });

  it("replays calls made before init in order", async () => {
    const analytics = await loadModule();
    analytics.track(EVENTS.downloadClicked, { from: "hero" });
    analytics.track(EVENTS.downloadClicked, { from: "footer" });

    const { client, sent } = fakeClient();
    analytics.flushQueue(client);

    expect(sent).toEqual([
      'download_clicked:{"from":"hero"}',
      'download_clicked:{"from":"footer"}',
    ]);
  });

  it("sends directly once the client is loaded", async () => {
    const analytics = await loadModule();
    const { client, sent } = fakeClient();
    analytics.flushQueue(client);

    analytics.track(EVENTS.downloadClicked);

    expect(sent).toEqual(["download_clicked"]);
  });

  it("drops calls past the queue limit and warns once", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const analytics = await loadModule();
    for (let i = 0; i < analytics.MAX_QUEUE + 5; i++) {
      analytics.track(EVENTS.downloadClicked);
    }

    const { client, sent } = fakeClient();
    analytics.flushQueue(client);

    expect(sent).toHaveLength(analytics.MAX_QUEUE);
    expect(warn).toHaveBeenCalledTimes(1);
  });

  it("initializes the SDK once with privacy defaults", async () => {
    const analytics = await loadModule();
    await analytics.initAnalytics();
    await analytics.initAnalytics();

    expect(sdk.init).toHaveBeenCalledTimes(1);
    expect(sdk.init).toHaveBeenCalledWith(
      "phc_test",
      expect.objectContaining({
        api_host: "https://us.i.posthog.com",
        persistence: "memory",
        autocapture: false,
        disable_session_recording: true,
      })
    );
  });
});
