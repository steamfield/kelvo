import { describe, expect, it } from "vitest";
import {
  type BlogEntry,
  blogPath,
  sortedPosts,
  relatedPosts,
  readingMinutes,
  blogPostingJsonLd,
  breadcrumbJsonLd,
} from "./blog";

function post(
  id: string,
  published: string,
  feature: BlogEntry["data"]["feature"] = "gpu",
  title = id
): BlogEntry {
  return {
    id,
    collection: "blog",
    body: "Test post",
    data: {
      title,
      description: "Test description",
      published: new Date(published),
      keyword: "test query",
      feature,
      relatedDocs: [],
    },
  };
}

describe("blog entries", () => {
  const oldest = post("old", "2026-01-01");
  const newest = post("new", "2026-03-01", "disk");
  const sameDay = post("a", "2026-03-01");
  it("sorts newest first and breaks date ties by title without changing input", () => {
    const input = [oldest, newest, sameDay];
    expect(sortedPosts(input).map((p) => p.id)).toEqual(["a", "new", "old"]);
    expect(input).toEqual([oldest, newest, sameDay]);
    expect(blogPath("guides/gpu")).toBe("/blog/guides/gpu");
  });
  it("prefers matching features, excludes itself, then uses publication order and limit", () => {
    const current = post("current", "2026-04-01");
    const entries = [newest, oldest, current, sameDay];
    expect(relatedPosts(entries, current).map((p) => p.id)).toEqual([
      "a",
      "old",
      "new",
    ]);
    expect(relatedPosts(entries, current, 1)).toEqual([sameDay]);
    expect(relatedPosts([current], current)).toEqual([]);
  });
  it("rounds reading time up at 230 words with a one-minute floor", () => {
    expect(readingMinutes("")).toBe(1);
    expect(readingMinutes("word ".repeat(230))).toBe(1);
    expect(readingMinutes("word ".repeat(231))).toBe(2);
    expect(readingMinutes("word ".repeat(460))).toBe(2);
  });
  it("describes the headline, dates, canonical and application without nullable fields", () => {
    const entry = post("gpu", "2026-03-01");
    const site = "https://statglass.steamfield.io";
    const data = blogPostingJsonLd(entry, site, "/og.webp");
    expect(data).toMatchObject({
      headline: "gpu",
      dateModified: data.datePublished,
      mainEntityOfPage: `${site}/blog/gpu`,
      image: `${site}/og.webp`,
      about: { "@id": `${site}/#app` },
      author: { name: "Steamfield" },
      keywords: "test query",
    });
    entry.data.headline = "A different heading";
    entry.data.updated = new Date("2026-04-01");
    expect(blogPostingJsonLd(entry, site, "/og.webp")).toMatchObject({
      headline: "A different heading",
      dateModified: "2026-04-01T00:00:00.000Z",
    });
    const crumbs = breadcrumbJsonLd(site, "GPU");
    expect(crumbs.itemListElement).toEqual([
      { "@type": "ListItem", position: 1, name: "Blog", item: `${site}/blog` },
      { "@type": "ListItem", position: 2, name: "GPU" },
    ]);
    const values = (value: unknown): unknown[] =>
      value !== null && typeof value === "object"
        ? Object.values(value).flatMap(values)
        : [value];
    expect(values([data, crumbs, breadcrumbJsonLd(site)])).not.toContain(null);
    expect(values([data, crumbs, breadcrumbJsonLd(site)])).not.toContain(
      undefined
    );
    expect(breadcrumbJsonLd(site).itemListElement).toHaveLength(1);
  });
});
