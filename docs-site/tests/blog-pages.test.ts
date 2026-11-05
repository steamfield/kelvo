// @vitest-environment node
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseFrontmatter } from "astro/markdown";
import { Window } from "happy-dom";
import { expect, inject, it } from "vitest";
import { DEMO_PARAM_KEYS, KNOWN_SCENARIOS } from "../src/lib/demo";

const site = "https://statglass.steamfield.io";
const blogDir = fileURLToPath(new URL("../src/content/blog/", import.meta.url));
const posts = readdirSync(blogDir, { recursive: true, encoding: "utf8" })
  .filter((name) => name.endsWith(".mdx"))
  .map((name) => ({
    id: name.slice(0, -4),
    ...parseFrontmatter(readFileSync(join(blogDir, name), "utf8")),
  }));
// Built once per run by tests/helpers/site-build-setup.ts.
const output = inject("siteDir");
const window = new Window({ settings: { disableJavaScriptEvaluation: true } });
const isFile = (path: string) => existsSync(path) && statSync(path).isFile();
const page = (path: string) =>
  new window.DOMParser().parseFromString(
    readFileSync(join(output, `${path}.html`), "utf8"),
    "text/html"
  );

function expectNoNulls(value: unknown) {
  expect(value).not.toBeNull();
  if (typeof value === "object" && value !== null) {
    for (const child of Object.values(value)) expectNoNulls(child);
  }
}

it("allows the word null in structured data but rejects nested null values", () => {
  expect(() =>
    expectNoNulls({ headline: "What a null reading means", items: ["null"] })
  ).not.toThrow();
  expect(() => expectNoNulls({ items: [{ headline: null }] })).toThrow();
});

it("publishes complete matching metadata and structured data for every blog page", () => {
  for (const post of [
    { id: "", frontmatter: { title: "Mac System Monitoring Guides", headline: "Blog" } },
    ...posts,
  ]) {
    const path = `/blog${post.id ? `/${post.id}` : ""}`;
    const doc = page(path);
    const meta = (key: string) =>
      doc
        .querySelector(`meta[name="${key}"], meta[property="${key}"]`)
        ?.getAttribute("content");
    expect(doc.querySelectorAll("h1"), path).toHaveLength(1);
    expect(doc.querySelector("h1")?.textContent).toBe(
      post.frontmatter.headline ?? post.frontmatter.title
    );
    expect(doc.title).toBe(`${post.frontmatter.title} | StatGlass`);
    expect(doc.title.length).toBeLessThanOrEqual(60);
    const description = meta("description") ?? "";
    expect(description.length).toBeGreaterThanOrEqual(120);
    expect(description.length).toBeLessThanOrEqual(160);
    expect(meta("og:description")).toBe(description);
    expect(meta("twitter:description")).toBe(description);
    expect(meta("og:title")).toBe(doc.title);
    expect(meta("twitter:title")).toBe(doc.title);
    expect(
      doc.querySelector('link[rel="canonical"]')?.getAttribute("href")
    ).toBe(`${site}${path}`);
    expect(
      doc
        .querySelector('link[type="application/rss+xml"]')
        ?.getAttribute("href")
    ).toBe(`${site}/blog/rss.xml`);
    expect(
      doc
        .querySelector('header nav[aria-label="Site"] a[href="/blog"]')
        ?.hasAttribute("aria-current")
    ).toBe(true);
    const scripts = doc.querySelectorAll('script[type="application/ld+json"]');
    expect(scripts).toHaveLength(1);
    for (const script of scripts) {
      const json = script.textContent ?? "";
      const data = JSON.parse(json);
      expectNoNulls(data);
      const types = data["@graph"].map(
        (node: Record<string, unknown>) => node["@type"]
      );
      expect(types).toContain("BreadcrumbList");
      expect(types).toContain(post.id ? "BlogPosting" : "CollectionPage");
      if ("published" in post.frontmatter) {
        const posting = data["@graph"].find(
          (node: Record<string, unknown>) => node["@type"] === "BlogPosting"
        );
        expect(posting.headline).toBe(doc.querySelector("h1")?.textContent);
        expect(posting.mainEntityOfPage).toBe(`${site}${path}`);
        expect(posting.datePublished).toBe(
          new Date(post.frontmatter.published).toISOString()
        );
        expect(posting.dateModified).toBe(
          new Date(
            post.frontmatter.updated ?? post.frontmatter.published
          ).toISOString()
        );
        expect(posting.about["@id"]).toBe(`${site}/#app`);
      }
    }
    if ("published" in post.frontmatter) {
      expect(meta("og:type")).toBe("article");
      expect(meta("article:published_time")).toBe(
        new Date(post.frontmatter.published).toISOString()
      );
    }
  }
});

it("lists every post in RSS, sitemap, llms and the index in publication order", () => {
  const rss = new window.DOMParser().parseFromString(
    readFileSync(join(output, "blog/rss.xml"), "utf8"),
    "application/xml"
  );
  const llms = readFileSync(join(output, "llms.txt"), "utf8");
  const sitemap = readdirSync(output)
    .filter((f) => /^sitemap-\d+\.xml$/.test(f))
    .map((f) => readFileSync(join(output, f), "utf8"))
    .join("");
  const expected = [...posts].sort(
    (a, b) =>
      new Date(b.frontmatter.published).getTime() -
        new Date(a.frontmatter.published).getTime() ||
      a.frontmatter.title.localeCompare(b.frontmatter.title)
  );
  expect(
    Array.from(page("/blog").querySelectorAll("main li h2 a"), (link) =>
      link.getAttribute("href")
    )
  ).toEqual(expected.map((p) => `/blog/${p.id}`));
  expect(rss.querySelectorAll("item")).toHaveLength(posts.length);
  for (const post of posts) {
    const url = `${site}/blog/${post.id}`;
    const item = Array.from(rss.querySelectorAll("item")).find(
      (item) => item.querySelector("link")?.textContent === url
    );
    expect(item, url).toBeTruthy();
    expect(item?.querySelector("title")?.textContent).toBe(
      post.frontmatter.title
    );
    expect(item?.querySelector("description")?.textContent).toBe(
      post.frontmatter.description
    );
    expect(
      new Date(item?.querySelector("pubDate")?.textContent ?? "").getTime()
    ).toBe(new Date(post.frontmatter.published).getTime());
    expect(sitemap).toContain(`<loc>${url}</loc>`);
    expect(llms).toContain(
      `- [${post.frontmatter.title}](${url}): ${post.frontmatter.description}`
    );
  }
});

it("rejects a directory as a link destination even when it contains built pages", () => {
  expect(isFile(join(output, "blog"))).toBe(false);
  expect(isFile(join(output, "blog.html"))).toBe(true);
});

it("resolves internal links, heading anchors and related docs", () => {
  for (const path of ["/blog", ...posts.map((p) => `/blog/${p.id}`)]) {
    const doc = page(path);
    for (const link of doc.querySelectorAll('a[href^="/"]')) {
      const url = new URL(link.getAttribute("href") ?? "", site);
      const target = url.pathname;
      const html = target === "/" ? "index.html" : `${target}.html`;
      expect(
        isFile(join(output, target)) || isFile(join(output, html)),
        `${path} -> ${url}`
      ).toBe(true);
      if (url.hash && isFile(join(output, html))) {
        const linked = new window.DOMParser().parseFromString(
          readFileSync(join(output, html), "utf8"),
          "text/html"
        );
        expect(
          linked.getElementById(decodeURIComponent(url.hash.slice(1)))
        ).toBeTruthy();
      }
    }
    for (const link of doc.querySelectorAll(
      'nav[aria-label="On this page"] a'
    )) {
      expect(
        doc.getElementById((link.getAttribute("href") ?? "").slice(1))?.tagName
      ).toBe("H2");
    }
  }
  for (const post of posts) {
    for (const id of post.frontmatter.relatedDocs) {
      expect(
        isFile(join(output, id === "index" ? "docs.html" : `docs/${id}.html`)),
        `${post.id}: ${id}`
      ).toBe(true);
    }
  }
});

it("uses only supported outer demo parameters and app scenarios", () => {
  const fixtures = readFileSync(
    new URL("../../src/core/mock/fixtures.ts", import.meta.url),
    "utf8"
  );
  const scenarios = /export const SCENARIOS = \[([\s\S]*?)\] as const/.exec(
    fixtures
  )?.[1];
  expect(scenarios).toBeTruthy();
  const names = Array.from(
    (scenarios ?? "").matchAll(/"([^"]+)"/g),
    (m) => m[1]
  );
  for (const scenario of KNOWN_SCENARIOS) expect(names).toContain(scenario);
  for (const post of posts) {
    for (const link of page(`/blog/${post.id}`).querySelectorAll("a")) {
      if (!link.textContent?.includes("Open the dashboard demo in its own tab"))
        continue;
      const url = new URL(link.getAttribute("href") ?? "", site);
      expect(url.pathname).toBe("/demo/index.html");
      expect(url.searchParams.get("window")).toBe("dashboard");
      for (const key of url.searchParams.keys())
        expect(["window", "route", ...DEMO_PARAM_KEYS]).toContain(key);
      if (url.searchParams.has("scenario"))
        expect(KNOWN_SCENARIOS).toContain(url.searchParams.get("scenario"));
    }
  }
});

it("renders the placeholder's MDX code frames, callout, table and literal CLI hyphens", () => {
  if (!posts.some((post) => post.id === "demo-kit")) return;
  const doc = page("/blog/demo-kit");
  expect(doc.querySelector("[data-code-title]")?.textContent).toBe(
    "Read recent history"
  );
  expect(doc.querySelector('[data-callout="note"]')).toBeTruthy();
  expect(doc.querySelector("[data-table-scroll] table")).toBeTruthy();
  const prose = Array.from(doc.querySelectorAll("p")).find((p) =>
    p.textContent?.startsWith("The prose flag")
  );
  expect(prose?.textContent).toContain("--since 1h");
  expect(prose?.querySelector("code")?.textContent).toBe("--since 1h");
  expect(prose?.textContent).not.toContain("–since");
  expect(doc.querySelector("[data-sf-agent-demo]")?.textContent).toContain(
    "statglass top --by cpu --limit 5"
  );
});
