// @vitest-environment node
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Window } from "happy-dom";
import { afterAll, beforeAll, expect, it } from "vitest";
import { buildSite } from "./helpers/frontend-build";

const site = "https://statglass.steamfield.io";
const docsDir = fileURLToPath(new URL("../src/content/docs/", import.meta.url));
const root = mkdtempSync(join(tmpdir(), "docs-pages-"));
const output = join(root, "site");
afterAll(() => rmSync(root, { recursive: true, force: true }));

beforeAll(async () => {
  const build = await buildSite(output);
  expect(build.status, build.stderr).toBe(0);
}, 180_000);

// Read straight from the source files, independently of the collection code under test.
const pages = readdirSync(docsDir, { recursive: true, encoding: "utf8" })
  .filter((name) => name.endsWith(".md"))
  .map((name) => {
    const id = name.replace(/\.md$/, "");
    const title =
      /^title: (.+)$/m.exec(readFileSync(join(docsDir, name), "utf8"))?.[1] ??
      "";
    return {
      id,
      title: title.replace(/^"(.*)"$/, "$1"),
      path: id === "index" ? "/docs" : `/docs/${id}`,
    };
  });

const window = new Window({ settings: { disableJavaScriptEvaluation: true } });
const documents = new Map<string, Document>();

/** The built HTML for a site path, or undefined when the build has no such page. */
function builtPage(path: string): Document | undefined {
  const cached = documents.get(path);
  if (cached) return cached;
  // format: "file" output: /docs -> docs.html, /docs/x -> docs/x.html.
  const file = join(output, path === "/" ? "index.html" : `${path}.html`);
  if (!existsSync(file)) return undefined;
  const document = new window.DOMParser().parseFromString(
    readFileSync(file, "utf8"),
    "text/html"
  );
  documents.set(path, document as unknown as Document);
  return document as unknown as Document;
}

function page(path: string): Document {
  const document = builtPage(path);
  if (!document) throw new Error(`no built page at ${path}`);
  return document;
}

it("builds every docs page with one h1 matching its title", () => {
  expect(pages.length).toBeGreaterThanOrEqual(6);
  for (const { path, title } of pages) {
    const headings = Array.from(
      page(path).querySelectorAll("h1"),
      (h1) => h1.textContent
    );
    expect(headings, path).toEqual([title]);
  }
});

it("marks exactly the current page in the docs navigation", () => {
  for (const { path } of pages) {
    const current = Array.from(
      page(path).querySelectorAll(
        "[data-docs-sidebar] nav[aria-label='Documentation'] [aria-current]"
      ),
      (link) => [link.getAttribute("href"), link.getAttribute("aria-current")]
    );
    expect(current, path).toEqual([[path, "page"]]);
  }
});

it("links every On this page entry to a heading on the page", () => {
  for (const { path } of pages) {
    const document = page(path);
    for (const link of document.querySelectorAll(
      "nav[aria-label='On this page'] a"
    )) {
      const id = decodeURIComponent((link.getAttribute("href") ?? "").slice(1));
      expect(document.getElementById(id)?.tagName, `${path}#${id}`).toMatch(
        /^H[23]$/
      );
    }
  }
});

it("resolves every site link in the docs prose to a built page and, with an anchor, its heading", () => {
  let checked = 0;
  for (const { path } of pages) {
    for (const link of page(path).querySelectorAll("article a[href^='/']")) {
      const href = link.getAttribute("href") ?? "";
      const [target = "", anchor] = href.split("#");
      const where = `${path} -> ${href}`;
      const linked = target.endsWith(".md")
        ? existsSync(join(output, target))
        : builtPage(target) !== undefined;
      expect(linked, where).toBe(true);
      if (anchor && !target.endsWith(".md")) {
        expect(
          page(target).getElementById(decodeURIComponent(anchor)),
          where
        ).not.toBeNull();
      }
      checked += 1;
    }
  }
  expect(checked).toBeGreaterThan(10);
});

it("publishes each page's Markdown beside it, linked from the page and starting with its title", () => {
  for (const { id, path, title } of pages) {
    const markdownPath = `/docs/${id}.md`;
    const alternate = page(path).querySelector(
      "link[rel='alternate'][type='text/markdown']"
    );
    expect(alternate?.getAttribute("href"), path).toBe(markdownPath);
    const markdown = readFileSync(join(output, markdownPath), "utf8");
    expect(markdown.startsWith(`# ${title}\n\n> `), markdownPath).toBe(true);
    expect(markdown, markdownPath).not.toMatch(/\]\(\/(?!\/)/);
  }
});

it("lists exactly the docs pages in llms.txt and every page in llms-full.txt", () => {
  const expected = pages.map(({ id }) => `${site}/docs/${id}.md`).sort();
  const llms = readFileSync(join(output, "llms.txt"), "utf8");
  const listed = Array.from(
    llms.matchAll(/\]\((https:\/\/[^)]+\/docs\/[^)]+\.md)\)/g),
    (m) => m[1]
  );
  expect(listed.sort()).toEqual(expected);
  const full = readFileSync(join(output, "llms-full.txt"), "utf8");
  const sources = Array.from(full.matchAll(/^Source: (\S+)$/gm), (m) => m[1]);
  expect(sources.sort()).toEqual(
    pages.map(({ path }) => `${site}${path}`).sort()
  );
});
