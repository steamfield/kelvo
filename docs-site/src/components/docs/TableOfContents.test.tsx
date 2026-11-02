import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { TableOfContents } from "./TableOfContents";

it("links each section and subsection of a page with three or more", () => {
  render(
    <TableOfContents
      headings={[
        { depth: 2, slug: "install", text: "Install" },
        { depth: 3, slug: "completions", text: "Completions" },
        { depth: 4, slug: "zsh", text: "zsh" },
        { depth: 2, slug: "build-it", text: "Build it" },
      ]}
    />
  );
  const links = screen.getAllByRole("link");
  expect(
    links.map((link) => [link.textContent, link.getAttribute("href")])
  ).toEqual([
    ["Install", "#install"],
    ["Completions", "#completions"],
    ["Build it", "#build-it"],
  ]);
  expect(
    screen.getByRole("navigation", { name: "On this page" })
  ).toBeVisible();
});

it("is left out of a page with fewer than three sections", () => {
  const { container } = render(
    <TableOfContents
      headings={[
        { depth: 2, slug: "a", text: "A" },
        { depth: 4, slug: "b", text: "B" },
        { depth: 2, slug: "c", text: "C" },
      ]}
    />
  );
  expect(container).toBeEmptyDOMElement();
});
