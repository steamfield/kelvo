export const docGroups = [
  "Getting started",
  "Using StatGlass",
  "Metrics",
  "Remote hosts",
  "Privacy",
  "Contributing",
] as const;
export type DocGroup = (typeof docGroups)[number];

export type DocMeta = {
  title: string;
  description: string;
  group: DocGroup;
  order: number;
  sidebarLabel?: string | undefined;
  featured?: boolean | undefined;
};

export type DocEntry = { id: string; data: DocMeta; body?: string | undefined };

export type DocLink = {
  id: string;
  href: string;
  label: string;
  description: string;
};
export type DocNavGroup = { title: DocGroup; items: DocLink[] };

export const docsSummary =
  "StatGlass is a free, open-source menu bar system monitor for Apple Silicon Macs. It shows CPU, GPU, memory, power and sensors, network, disk and battery once a second, and keeps local history.";

// The site builds with trailingSlash "never" and file-style output, so pages
// live at /docs and /docs/<id> (docs.html, docs/<id>.html).
export function docPath(id: string): string {
  return id === "index" ? "/docs" : `/docs/${id}`;
}

export function markdownPath(id: string): string {
  return `/docs/${id}.md`;
}

function link(entry: DocEntry): DocLink {
  return {
    id: entry.id,
    href: docPath(entry.id),
    label: entry.data.sidebarLabel ?? entry.data.title,
    description: entry.data.description,
  };
}

function byOrder(a: DocEntry, b: DocEntry): number {
  return a.data.order - b.data.order || a.id.localeCompare(b.id);
}

/** The sidebar: groups in reading order, each page by its `order`, ties by id. Empty groups are left out. */
export function docsNav(entries: readonly DocEntry[]): DocNavGroup[] {
  return docGroups
    .map((title) => ({
      title,
      items: entries
        .filter((entry) => entry.data.group === title)
        .sort(byOrder)
        .map(link),
    }))
    .filter((group) => group.items.length > 0);
}

/** The pages before and after this one in sidebar order, and the group it sits in. */
export function pageLinks(
  entries: readonly DocEntry[],
  id: string
): {
  group: DocGroup | undefined;
  previous: DocLink | undefined;
  next: DocLink | undefined;
} {
  const nav = docsNav(entries);
  const order = nav.flatMap((group) => group.items);
  const index = order.findIndex((item) => item.id === id);
  return {
    group: nav.find((group) => group.items.some((item) => item.id === id))
      ?.title,
    previous: index > 0 ? order[index - 1] : undefined,
    next: index >= 0 ? order[index + 1] : undefined,
  };
}

/** Featured pages in sidebar order: the cards on the docs home. */
export function featuredDocs(entries: readonly DocEntry[]): DocLink[] {
  const featured = new Set(
    entries.filter((entry) => entry.data.featured).map((entry) => entry.id)
  );
  return docsNav(entries)
    .flatMap((group) => group.items)
    .filter((item) => featured.has(item.id));
}

const siteLink = /\]\((\/[^)\s]*)\)/g;

function markdownTarget(path: string, site: string): string {
  const [pathname = "", hash] = path.split("#");
  const docs = /^\/docs(?:\/(.+?))?\/?$/.exec(pathname);
  const target = docs ? markdownPath(docs[1] ?? "index") : pathname;
  return `${new URL(target, site).href}${hash === undefined ? "" : `#${hash}`}`;
}

// Site-relative links become absolute, and links to docs pages point at their Markdown, so a page
// pasted into an agent's context still resolves. Code blocks are copied as written.
function absoluteLinks(body: string, site: string): string {
  let fenced = false;
  return body
    .split("\n")
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) fenced = !fenced;
      if (fenced) return line;
      return line.replace(
        siteLink,
        (_, path: string) => `](${markdownTarget(path, site)})`
      );
    })
    .join("\n");
}

function linkList(links: readonly DocLink[], site: string): string {
  return links
    .map(
      (item) =>
        `- [${item.label}](${markdownTarget(item.href, site)}): ${item.description}`
    )
    .join("\n");
}

/** One page as Markdown for agents: its title and description, then the source body. */
export function markdownExport(
  entry: DocEntry,
  entries: readonly DocEntry[],
  site: string
): string {
  const parts = [`# ${entry.data.title}`, `> ${entry.data.description}`];
  const body = (entry.body ?? "").trim();
  if (body) parts.push(absoluteLinks(body, site));
  if (entry.id === "index")
    parts.push(`## Start here\n\n${linkList(featuredDocs(entries), site)}`);
  return `${parts.join("\n\n")}\n`;
}

/** The llms.txt index (llmstxt.org): every docs page by group, as Markdown URLs. */
export function llmsTxt(entries: readonly DocEntry[], site: string): string {
  const absolute = (path: string) => new URL(path, site).href;
  const groups = docsNav(entries).map(
    (group) => `## ${group.title}\n\n${linkList(group.items, site)}`
  );
  return `${[
    "# StatGlass",
    `> ${docsSummary}`,
    `Every docs page is also Markdown: add \`.md\` to its path, as in ${absolute(markdownPath("install"))}. ${absolute("/llms-full.txt")} has every page in one file.`,
    ...groups,
    [
      "## Optional",
      "",
      "- [Source code](https://github.com/steamfield/statglass): the app, its Rust engine and this site",
    ].join("\n"),
  ].join("\n\n")}\n`;
}

/** Every docs page in sidebar order, in one file. */
export function llmsFull(entries: readonly DocEntry[], site: string): string {
  const byId = new Map(entries.map((entry) => [entry.id, entry]));
  const pages = docsNav(entries)
    .flatMap((group) => group.items)
    .map((item) => byId.get(item.id))
    .filter((entry) => entry !== undefined)
    .map(
      (entry) =>
        `Source: ${new URL(docPath(entry.id), site).href}\n\n${markdownExport(entry, entries, site).trim()}`
    );
  return `${pages.join("\n\n---\n\n")}\n`;
}
