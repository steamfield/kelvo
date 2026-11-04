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

// buildSite passes no PostHog key, and the layout only calls initAnalytics, and only
// creates the consent store and banner, with one, so the build bundles none of it.
// The banner's stylesheet ships either way, with the other library CSS.

// Every JavaScript file a page loads or may load: its inline scripts, script tags
// and island components, renderers and before-hydration scripts, then the imports
// (static, and dynamic with any quote style) of each file they reach. Ported from
// stacks-api's analytics build test.
function scriptsReachedFrom(root: string, page: string): Map<string, string> {
  const html = readFileSync(join(root, page), "utf8");
  const reached = new Map<string, string>();
  const pending: string[] = [];
  const follow = (source: string, folder: string) => {
    // A `//host/...` target is another origin; it isn't in the build.
    for (const m of source.matchAll(
      /["'`](\.\/|\/(?!\/))([^"'`]+\.js)["'`]/g
    )) {
      const target = m[2] ?? "";
      // A template such as posthog-js's `/static/${version}/${name}.js` is a URL
      // built at run time for its CDN scripts, not a file in the build.
      if (target.includes("${")) continue;
      pending.push(m[1] === "/" ? target : folder + target);
    }
  };
  const pageFolder = page.slice(0, page.lastIndexOf("/") + 1);
  const inline = html.matchAll(
    /<script(?![^>]*\ssrc=)[^>]*>([\s\S]*?)<\/script>/g
  );
  for (const [index, m] of Array.from(inline).entries()) {
    const source = m[1] ?? "";
    reached.set(`${page}#inline-${index}`, source);
    follow(source, pageFolder);
  }
  const entries =
    /(?:<script[^>]*\ssrc|\scomponent-url|\srenderer-url|\sbefore-hydration-url)="\/(?!\/)([^"]+\.js)"/g;
  pending.push(...Array.from(html.matchAll(entries), (m) => m[1] ?? ""));
  for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
    if (reached.has(path)) continue;
    const source = readFileSync(join(root, path), "utf8");
    reached.set(path, source);
    follow(source, path.slice(0, path.lastIndexOf("/") + 1));
  }
  return reached;
}

// The stylesheets a page links, by path, with their contents.
function stylesheetsOf(root: string, page: string): Map<string, string> {
  const html = readFileSync(join(root, page), "utf8");
  const links = html.matchAll(
    /<link[^>]*\srel="stylesheet"[^>]*\shref="\/([^"]+)"/g
  );
  return new Map(
    Array.from(links, (m) => m[1] ?? "").map((path) => [
      path,
      readFileSync(join(root, path), "utf8"),
    ])
  );
}

// The paths, not the contents, so a failure prints something readable.
const filesContaining = (reached: Map<string, string>, text: string) =>
  [...reached]
    .filter(([, source]) => source.includes(text))
    .map(([path]) => path);

// The default ingest host: in the analytics module and in posthog-js, nowhere else.
const analyticsMarker = "us.i.posthog.com";
// posthog-js's consent record prefix: in the SDK only, so finding it means the
// scanner followed the module's dynamic import of posthog-js.
const sdkMarker = "__ph_opt_in_out_";
// The consent store's cookie name: in @steamfield/ui/consent only.
const consentMarker = "sf-consent";
// A rule from @steamfield/ui/consent's stylesheet, and a variable it reads (which
// the site's theme also sets).
const bannerStyleMarker = "--sf-consent-background";
const isConsentStyle = (source: string) =>
  /\[data-part=["']?consent["']?\]/.test(source);

it("loads no analytics or consent code in a build without a PostHog key", () => {
  const reached = scriptsReachedFrom(output, "index.html");
  expect(reached.size).toBeGreaterThan(0);
  expect(filesContaining(reached, analyticsMarker)).toEqual([]);
  expect(filesContaining(reached, consentMarker)).toEqual([]);
});

it("loads the key once, posthog-js, the banner and its styles when the build has a key", async () => {
  // Not a credential: any non-empty value turns analytics on for the build.
  const key = "statglass-build-test-key";
  const keyed = join(root, "keyed");
  const build = await buildSite(keyed, { PUBLIC_POSTHOG_KEY: key });
  expect(build.status, build.stderr).toBe(0);
  const reached = scriptsReachedFrom(keyed, "index.html");
  expect(filesContaining(reached, key)).toHaveLength(1);
  expect(filesContaining(reached, sdkMarker)).toHaveLength(1);
  expect(filesContaining(reached, consentMarker)).not.toEqual([]);
  // The site value the layout passes to initAnalytics, and the consent domain.
  const sources = [...reached.values()];
  expect(
    sources.some((source) => /site:\s*["'`]statglass["'`]/.test(source))
  ).toBe(true);
  expect(sources.some((source) => source.includes("steamfield.io"))).toBe(true);
  const styles = [...stylesheetsOf(keyed, "index.html").values()];
  expect(styles.some((source) => source.includes(bannerStyleMarker))).toBe(
    true
  );
  expect(styles.some(isConsentStyle)).toBe(true);
}, 180_000);

it("links the shared privacy page from the footer and the docs privacy page", () => {
  const footerLink = 'footer a[href="https://steamfield.io/privacy"]';
  expect(builtPage("/")?.querySelector(footerLink)).toBeTruthy();
  const docs = builtPage("/docs/privacy");
  expect(
    docs?.querySelector('main a[href="https://steamfield.io/privacy"]')
  ).toBeTruthy();
});
