import { callouts } from "./callouts";
import { codeBlocks, codeGroups } from "./code-blocks";
import { codeMeta } from "./code-meta";
import { headingAnchors } from "./heading-anchors";
import { tables } from "./tables";

// Token colors come from --astro-code-* variables, which DocsProse.module.css maps to site tokens.
export const docsShikiConfig = {
  theme: "css-variables" as const,
  transformers: [codeMeta()],
};

// Factories: Sätteri calls each once per document, so counters and slugs restart on every page.
export const docsHastPlugins = [
  codeBlocks,
  codeGroups,
  callouts,
  headingAnchors,
  tables,
];
