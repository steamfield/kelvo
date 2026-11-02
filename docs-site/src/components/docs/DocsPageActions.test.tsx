import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DocsPageActions } from "./DocsPageActions";

const markdownUrl = "https://example.com/docs/install.md";

it("offers the page's Markdown to copy, to view and to hand to a chat", () => {
  const { container } = render(
    <DocsPageActions
      markdown={"# Install StatGlass\n\nDownload it."}
      markdownHref="/docs/install.md"
      markdownUrl={markdownUrl}
    />
  );
  expect(container.querySelector("pre[hidden]")?.textContent).toBe(
    "# Install StatGlass\n\nDownload it."
  );
  expect(container.querySelector("button[data-copy-command]")).toHaveAttribute(
    "aria-controls",
    "docs-page-markdown"
  );
  expect(
    screen.getByRole("link", { name: "View as Markdown" })
  ).toHaveAttribute("href", "/docs/install.md");
  const question = `Read ${markdownUrl} and answer my questions about StatGlass.`;
  for (const [name, base] of [
    ["Open in Claude", "https://claude.ai/new"],
    ["Open in ChatGPT", "https://chatgpt.com/"],
  ] as const) {
    const href = screen.getByRole("link", { name }).getAttribute("href") ?? "";
    const url = new URL(href);
    expect(`${url.origin}${url.pathname}`).toBe(base);
    expect(url.searchParams.get("q")).toBe(question);
  }
});
