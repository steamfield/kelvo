import { describe, expect, it } from "vitest";
import {
  type DocEntry,
  type DocMeta,
  docPath,
  docsNav,
  featuredDocs,
  llmsFull,
  llmsTxt,
  markdownExport,
  markdownPath,
  pageLinks,
} from "./docs";

const site = "https://example.com";

function entry(id: string, data: Partial<DocMeta> = {}, body = ""): DocEntry {
  return {
    id,
    body,
    data: {
      title: `Title ${id}`,
      description: `About ${id}`,
      group: "Getting started",
      order: 1,
      ...data,
    },
  };
}

const entries = [
  entry("reference/output", { group: "Metrics", order: 2 }),
  entry("index", { title: "Introduction", order: 0 }),
  entry("concepts/verify", { group: "Using StatGlass", order: 3 }),
  entry("quickstart", { title: "Quickstart", order: 1, featured: true }),
  entry("concepts", {
    group: "Using StatGlass",
    order: 1,
    sidebarLabel: "Overview",
    featured: true,
  }),
  entry("agents", { order: 1 }),
];

describe("paths", () => {
  it("maps the docs home and nested pages to their HTML and Markdown paths", () => {
    expect(docPath("index")).toBe("/docs");
    expect(docPath("reference/commands")).toBe("/docs/reference/commands");
    expect(markdownPath("index")).toBe("/docs/index.md");
    expect(markdownPath("reference/commands")).toBe(
      "/docs/reference/commands.md"
    );
  });
});

describe("docsNav", () => {
  it("orders groups for reading and pages by order, breaking ties on id", () => {
    expect(
      docsNav(entries).map((group) => [
        group.title,
        group.items.map((item) => item.id),
      ])
    ).toEqual([
      ["Getting started", ["index", "agents", "quickstart"]],
      ["Using StatGlass", ["concepts", "concepts/verify"]],
      ["Metrics", ["reference/output"]],
    ]);
  });

  it("labels a page with its sidebar label when it has one", () => {
    const concepts = docsNav(entries)[1]?.items[0];
    expect(concepts).toEqual({
      id: "concepts",
      href: "/docs/concepts",
      label: "Overview",
      description: "About concepts",
    });
  });

  it("is empty with no pages", () => {
    expect(docsNav([])).toEqual([]);
  });
});

describe("pageLinks", () => {
  it("links across a group boundary", () => {
    const links = pageLinks(entries, "quickstart");
    expect(links.group).toBe("Getting started");
    expect(links.previous?.id).toBe("agents");
    expect(links.next?.id).toBe("concepts");
  });

  it("has no previous page on the first page and no next page on the last", () => {
    expect(pageLinks(entries, "index").previous).toBeUndefined();
    expect(pageLinks(entries, "reference/output").next).toBeUndefined();
  });

  it("finds nothing for an unknown page", () => {
    expect(pageLinks(entries, "missing")).toEqual({
      group: undefined,
      previous: undefined,
      next: undefined,
    });
  });
});

describe("featuredDocs", () => {
  it("lists featured pages in sidebar order", () => {
    expect(featuredDocs(entries).map((item) => item.id)).toEqual([
      "quickstart",
      "concepts",
    ]);
  });
});

describe("markdownExport", () => {
  it("starts with the title and description and keeps the body", () => {
    const page = entry("agents", {}, "Run `stacks guide`.");
    expect(markdownExport(page, entries, site)).toBe(
      "# Title agents\n\n> About agents\n\nRun `stacks guide`.\n"
    );
  });

  it("points docs links at their Markdown and makes other site links absolute", () => {
    const page = entry(
      "agents",
      {},
      "See [verify](/docs/concepts/verify#outcomes), [home](/docs), [privacy](/privacy) and [GitHub](https://github.com/steamfield/statglass)."
    );
    expect(markdownExport(page, entries, site)).toContain(
      "See [verify](https://example.com/docs/concepts/verify.md#outcomes), [home](https://example.com/docs/index.md), [privacy](https://example.com/privacy) and [GitHub](https://github.com/steamfield/statglass)."
    );
  });

  it("leaves links inside code blocks as written", () => {
    const body =
      "```md\n[verify](/docs/concepts/verify)\n```\n\n[verify](/docs/concepts/verify)";
    const output = markdownExport(entry("agents", {}, body), entries, site);
    expect(output).toContain("```md\n[verify](/docs/concepts/verify)\n```");
    expect(output).toContain(
      "\n[verify](https://example.com/docs/concepts/verify.md)"
    );
  });

  it("lists the featured pages on the docs home", () => {
    const home = entries.find((page) => page.id === "index");
    if (!home) throw new Error("fixture has no home page");
    expect(markdownExport(home, entries, site)).toBe(
      [
        "# Introduction",
        "",
        "> About index",
        "",
        "## Start here",
        "",
        "- [Quickstart](https://example.com/docs/quickstart.md): About quickstart",
        "- [Overview](https://example.com/docs/concepts.md): About concepts",
        "",
      ].join("\n")
    );
  });
});

describe("llms files", () => {
  it("lists every page's Markdown URL under its group", () => {
    const lines = llmsTxt(entries, site).split("\n");
    const pageLines = lines.filter(
      (line) => line.startsWith("- [") && line.includes("/docs/")
    );
    expect(pageLines).toEqual([
      "- [Introduction](https://example.com/docs/index.md): About index",
      "- [Title agents](https://example.com/docs/agents.md): About agents",
      "- [Quickstart](https://example.com/docs/quickstart.md): About quickstart",
      "- [Overview](https://example.com/docs/concepts.md): About concepts",
      "- [Title concepts/verify](https://example.com/docs/concepts/verify.md): About concepts/verify",
      "- [Title reference/output](https://example.com/docs/reference/output.md): About reference/output",
    ]);
    expect(lines[0]).toBe("# StatGlass");
    expect(lines.filter((line) => line.startsWith("## "))).toEqual([
      "## Getting started",
      "## Using StatGlass",
      "## Metrics",
      "## Optional",
    ]);
  });

  it("joins every page in sidebar order with its source URL", () => {
    const sources = llmsFull(entries, site)
      .split("\n")
      .filter((line) => line.startsWith("Source: "));
    expect(sources).toEqual([
      "Source: https://example.com/docs",
      "Source: https://example.com/docs/agents",
      "Source: https://example.com/docs/quickstart",
      "Source: https://example.com/docs/concepts",
      "Source: https://example.com/docs/concepts/verify",
      "Source: https://example.com/docs/reference/output",
    ]);
  });
});
