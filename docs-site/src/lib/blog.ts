import type { CollectionEntry } from "astro:content";

export type BlogEntry = CollectionEntry<"blog">;
export const blogPath = (id: string) => `/blog/${id}`;

export function sortedPosts<T extends BlogEntry>(entries: readonly T[]): T[] {
  return [...entries].sort(
    (a, b) =>
      b.data.published.getTime() - a.data.published.getTime() ||
      a.data.title.localeCompare(b.data.title)
  );
}

export function relatedPosts<T extends BlogEntry>(
  entries: readonly T[],
  entry: BlogEntry,
  n = 3
): T[] {
  return sortedPosts(entries.filter((post) => post.id !== entry.id))
    .sort(
      (a, b) =>
        Number(b.data.feature === entry.data.feature) -
        Number(a.data.feature === entry.data.feature)
    )
    .slice(0, n);
}

export function readingMinutes(body: string): number {
  return Math.max(1, Math.ceil((body.trim().match(/\S+/g)?.length ?? 0) / 230));
}

export function blogPostingJsonLd(
  entry: BlogEntry,
  site: string | URL,
  ogImage: string
) {
  const url = new URL(blogPath(entry.id), site).href;
  const organization = {
    "@type": "Organization",
    name: "Steamfield",
    url: "https://steamfield.io",
  };
  return {
    "@type": "BlogPosting",
    headline: entry.data.headline ?? entry.data.title,
    description: entry.data.description,
    datePublished: entry.data.published.toISOString(),
    dateModified: (entry.data.updated ?? entry.data.published).toISOString(),
    url,
    mainEntityOfPage: url,
    image: new URL(ogImage, site).href,
    author: organization,
    publisher: organization,
    about: {
      "@type": "SoftwareApplication",
      "@id": new URL("/#app", site).href,
      name: "StatGlass",
    },
    keywords: entry.data.keyword,
  };
}

export function breadcrumbJsonLd(site: string | URL, name?: string) {
  return {
    "@type": "BreadcrumbList",
    itemListElement: [
      {
        "@type": "ListItem",
        position: 1,
        name: "Blog",
        ...(name ? { item: new URL("/blog", site).href } : {}),
      },
      ...(name ? [{ "@type": "ListItem", position: 2, name }] : []),
    ],
  };
}

export const blogDescription =
  "Mac system monitoring guides with interactive StatGlass demos. Explore CPU, GPU, memory, disk and network readings, and learn to use local history.";
