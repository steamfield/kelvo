import type { ShikiTransformer } from "shiki";

export type FenceMeta = { title?: string; group?: string };

const pair = /([a-z]+)="([^"]*)"/g;

/** Reads `title="…"` and `group="…"` from a code fence's info string; other keys are ignored. */
export function parseFenceMeta(raw: string | undefined): FenceMeta {
  const meta: FenceMeta = {};
  for (const [, key, value] of (raw ?? "").matchAll(pair)) {
    if ((key === "title" || key === "group") && value) meta[key] = value;
  }
  return meta;
}

/** Carries fence meta onto Shiki's `<pre>`: Sätteri swaps the fence for Shiki's output before any hast plugin runs. */
export function codeMeta(): ShikiTransformer {
  return {
    name: "docs-code-meta",
    pre(node) {
      const { title, group } = parseFenceMeta(this.options.meta?.__raw);
      if (title) node.properties["data-title"] = title;
      if (group) node.properties["data-group"] = group;
    },
  };
}
