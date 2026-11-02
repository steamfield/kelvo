import type { PostHogInterface } from "posthog-js";
import type { EventName } from "@/lib/events";

/**
 * The only module that touches posthog-js. With PUBLIC_POSTHOG_KEY empty (the
 * normal state in dev, tests and CI) the SDK never loads and every export is a
 * silent no-op.
 */

type Props = Record<string, unknown>;
/** The SDK surface this module uses. */
type Client = Pick<PostHogInterface, "capture">;
type Call = { event: EventName; props?: Props | undefined };

export const MAX_QUEUE = 50;
const DEFAULT_HOST = "https://us.i.posthog.com";

let client: Client | null = null;
let initStarted = false;
let queue: Call[] = [];
let warnedOverflow = false;

function key(): string {
  return import.meta.env.PUBLIC_POSTHOG_KEY ?? "";
}

function enabled(): boolean {
  return typeof window !== "undefined" && key() !== "";
}

function run(ph: Client, call: Call) {
  ph.capture(call.event, call.props);
}

function dispatch(call: Call) {
  if (!enabled()) return;
  if (client) {
    run(client, call);
    return;
  }
  if (queue.length >= MAX_QUEUE) {
    if (!warnedOverflow) {
      console.warn(
        `analytics: dropped calls made before init (queue limit ${MAX_QUEUE})`
      );
      warnedOverflow = true;
    }
    return;
  }
  queue.push(call);
}

/** Replays queued calls in order. Exported for tests. */
export function flushQueue(ph: Client) {
  client = ph;
  const pending = queue;
  queue = [];
  for (const call of pending) run(ph, call);
}

export async function initAnalytics(): Promise<void> {
  if (!enabled() || initStarted) return;
  initStarted = true;
  const { default: posthog } = await import("posthog-js");
  posthog.init(key(), {
    api_host: import.meta.env.PUBLIC_POSTHOG_HOST || DEFAULT_HOST,
    person_profiles: "identified_only",
    persistence: "memory",
    autocapture: false,
    disable_session_recording: true,
    loaded: flushQueue,
  });
}

export function track(event: EventName, props?: Props) {
  dispatch({ event, props });
}
