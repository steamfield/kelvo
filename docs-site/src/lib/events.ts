/**
 * Every analytics event the site sends. `track()` only accepts these names.
 * Naming: snake_case object_verb in the past tense (`download_clicked`), with
 * snake_case properties.
 */
export const EVENTS = {
  downloadClicked: "download_clicked",
} as const;

export type EventName = (typeof EVENTS)[keyof typeof EVENTS];
