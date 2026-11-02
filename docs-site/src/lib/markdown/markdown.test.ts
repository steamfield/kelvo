import { createSatteriMarkdownProcessor } from "@astrojs/markdown-satteri";
import { describe, expect, it } from "vitest";
import {
  createHastPlugins,
  createShikiConfig,
  parseFenceMeta,
} from "@steamfield/ui/markdown";

// The same processor and plugins astro.config.mjs hands the docs collection.
const processor = await createSatteriMarkdownProcessor({
  syntaxHighlight: "shiki",
  shikiConfig: createShikiConfig(),
  hastPlugins: createHastPlugins(),
  features: { smartPunctuation: false },
});

async function render(markdown: string) {
  const result = await processor.render(markdown, {});
  const root = document.createElement("div");
  root.innerHTML = result.code;
  return { root, headings: result.metadata.headings };
}

const fence = (code: string, meta = "", lang = "sh") =>
  `\`\`\`${lang} ${meta}\n${code}\n\`\`\``;

describe("fence meta", () => {
  it("reads title and group and ignores other keys", () => {
    expect(parseFenceMeta('title="Terminal" group="new" theme="x"')).toEqual({
      title: "Terminal",
      group: "new",
    });
  });

  it("returns nothing for a fence without meta", () => {
    expect(parseFenceMeta(undefined)).toEqual({});
  });
});

describe("code blocks", () => {
  it("captions a titled block and pairs it with a copy button for its code", async () => {
    const { root } = await render(fence("stacks verify", 'title="Terminal"'));
    const figure = root.querySelector("figure");
    const pre = figure?.querySelector("pre");
    const button = figure?.querySelector("button");
    expect(figure?.querySelector("figcaption")?.textContent).toBe("Terminal");
    expect(pre?.textContent).toBe("stacks verify");
    expect(button?.getAttribute("aria-controls")).toBe(pre?.id);
    expect(button?.getAttribute("aria-label")).toBe("Copy code");
    expect(button?.hidden).toBe(true);
  });

  it("frames an untitled block without a caption", async () => {
    const { root } = await render(fence("stacks doctor"));
    expect(root.querySelectorAll("figure")).toHaveLength(1);
    expect(root.querySelector("figcaption")).toBeNull();
  });

  it("gives every block on a page its own id, including one inside a list", async () => {
    const { root } = await render(
      [
        fence("one"),
        "1. Step",
        `   ${fence("two").replaceAll("\n", "\n   ")}`,
        "",
        fence("three"),
      ].join("\n\n")
    );
    const ids = [...root.querySelectorAll("pre")].map((pre) => pre.id);
    expect(ids).toEqual(["code-1", "code-2", "code-3"]);
    expect(root.querySelector("li figure pre")?.textContent).toBe("two");
  });

  it("restarts ids on the next page", async () => {
    await render(fence("first page"));
    const { root } = await render(fence("second page"));
    expect(root.querySelector("pre")?.id).toBe("code-1");
  });
});

describe("code groups", () => {
  it("joins adjacent blocks that share a group, in order", async () => {
    const { root } = await render(
      [
        fence("stacks new go-cli demo", 'title="Command" group="new"'),
        fence(
          "Wrote demo/.stacks/BRIEF.md",
          'title="Output" group="new"',
          "text"
        ),
      ].join("\n\n")
    );
    const positions = [...root.querySelectorAll("figure")].map((figure) =>
      figure.getAttribute("data-group-position")
    );
    expect(positions).toEqual(["first", "last"]);
  });

  it("marks the blocks between the ends of a longer run as middle", async () => {
    const blocks = ["a", "b", "c"].map((code) => fence(code, 'group="x"'));
    const { root } = await render(blocks.join("\n\n"));
    const positions = [...root.querySelectorAll("figure")].map((figure) =>
      figure.getAttribute("data-group-position")
    );
    expect(positions).toEqual(["first", "middle", "last"]);
  });

  it("does not join blocks separated by a paragraph", async () => {
    const { root } = await render(
      [fence("a", 'group="x"'), "Then:", fence("b", 'group="x"')].join("\n\n")
    );
    const positions = [...root.querySelectorAll("figure")].map((figure) =>
      figure.getAttribute("data-group-position")
    );
    expect(positions).toEqual([null, null]);
  });

  it("does not join neighbours from different groups", async () => {
    const { root } = await render(
      [fence("a", 'group="x"'), fence("b", 'group="y"')].join("\n\n")
    );
    expect(root.querySelectorAll("[data-group-position]")).toHaveLength(0);
  });
});

describe("callouts", () => {
  it.each([
    ["NOTE", "note", "Note"],
    ["TIP", "tip", "Tip"],
    ["WARNING", "warning", "Warning"],
    ["IMPORTANT", "important", "Important"],
  ])("renders > [!%s] as a labelled note", async (marker, kind, title) => {
    const { root } = await render(`> [!${marker}]\n> Run **verify** first.`);
    const callout = root.querySelector('[role="note"]');
    expect(callout?.getAttribute("data-callout")).toBe(kind);
    expect(callout?.querySelector("[data-callout-title]")?.textContent).toBe(
      title
    );
    expect(callout?.textContent).not.toContain(`[!${marker}]`);
    expect(callout?.querySelector("strong")?.textContent).toBe("verify");
  });

  it("drops the marker's own paragraph when the body starts in the next one", async () => {
    const { root } = await render("> [!TIP]\n>\n> Body text.");
    const paragraphs = [
      ...(root.querySelectorAll('[role="note"] p') ?? []),
    ].map((p) => p.textContent);
    expect(paragraphs).toEqual(["Tip", "Body text."]);
  });

  it("leaves an unknown marker as a plain quote", async () => {
    const { root } = await render("> [!FOO]\n> Text.");
    expect(root.querySelector("blockquote")?.getAttribute("role")).toBeNull();
  });

  it("leaves a marker that isn't at the start as a plain quote", async () => {
    const { root } = await render("> Text and [!NOTE] later.");
    expect(root.querySelector('[role="note"]')).toBeNull();
  });
});

describe("headings", () => {
  it("links each section heading to itself and reports clean heading text", async () => {
    const { root, headings } = await render(
      "## Run `verify`\n\n### Read the report"
    );
    expect(headings).toEqual([
      { depth: 2, slug: "run-verify", text: "Run verify" },
      { depth: 3, slug: "read-the-report", text: "Read the report" },
    ]);
    const anchor = root.querySelector("h2 a[data-heading-anchor]");
    expect(anchor?.getAttribute("href")).toBe("#run-verify");
    expect(root.querySelector("h2")?.id).toBe("run-verify");
  });

  it("numbers a repeated heading", async () => {
    const { headings } = await render("## Examples\n\n## Examples");
    expect(headings.map((heading) => heading.slug)).toEqual([
      "examples",
      "examples-1",
    ]);
  });
});

describe("tables", () => {
  it("puts a table in a focusable scroll box with column headers", async () => {
    const { root } = await render(
      "| Flag | Meaning |\n| --- | --- |\n| `--json` | One result |"
    );
    const box = root.querySelector("[data-table-scroll]");
    expect(box?.getAttribute("tabindex")).toBe("0");
    expect(
      [...(box?.querySelectorAll("th") ?? [])].map((th) =>
        th.getAttribute("scope")
      )
    ).toEqual(["col", "col"]);
  });
});

describe("prose", () => {
  it("keeps flags intact instead of turning -- into a dash", async () => {
    const { root } = await render("Add --json to any command.");
    expect(root.textContent).toContain("--json");
  });
});
