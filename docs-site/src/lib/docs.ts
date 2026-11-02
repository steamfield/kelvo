import { createDocs } from "@steamfield/ui/docs";
import type { DocEntry as LibraryDocEntry } from "@steamfield/ui/docs";

export type { DocLink, DocNavGroup } from "@steamfield/ui/docs";

export const docGroups = [
  "Getting started",
  "Using StatGlass",
  "Metrics",
  "Remote hosts",
  "Privacy",
  "Contributing",
] as const;
export type DocGroup = (typeof docGroups)[number];

export type DocMeta = LibraryDocEntry["data"] & { group: DocGroup };
export type DocEntry = LibraryDocEntry & { data: DocMeta };

export const docsSummary =
  "StatGlass is a free, open-source menu bar system monitor for Apple Silicon Macs. It shows CPU, GPU, memory, power and sensors, network, disk and battery once a second, and keeps local history.";

// The site builds with trailingSlash "never" and file-style output, so pages
// live at /docs and /docs/<id> (docs.html, docs/<id>.html).
const docs = createDocs({
  name: "StatGlass",
  summary: docsSummary,
  groups: docGroups,
  trailingSlash: "never",
  exampleId: "install",
  optionalLinks: [
    {
      label: "Source code",
      href: "https://github.com/steamfield/statglass",
      description: "the app, its Rust engine and this site",
    },
  ],
});

export const docPath = docs.docPath;
export const markdownPath = docs.markdownPath;
export const docsNav = docs.docsNav;
export const pageLinks = docs.pageLinks;
export const featuredDocs = docs.featuredDocs;
export const markdownExport = docs.pageMarkdown;
export const llmsTxt = docs.llmsIndex;
export const llmsFull = docs.llmsFull;
