/**
 * Every analytics event the site sends. `track()` only accepts these names.
 * Naming: snake_case object_verb in the past tense (`download_clicked`), with
 * snake_case properties.
 */
export const EVENTS = {
  downloadClicked: "download_clicked",
  demoStarted: "demo_started",
  demoDashboardOpened: "demo_dashboard_opened",
} as const;

export type EventName = (typeof EVENTS)[keyof typeof EVENTS];
export type EventProps = Record<string, string | number | boolean | null>;
