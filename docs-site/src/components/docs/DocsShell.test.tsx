import { render, screen, within } from "@testing-library/react";
import { expect, it } from "vitest";
import type { DocNavGroup } from "../../lib/docs";
import { DocsShell } from "./DocsShell";

const page = {
  id: "quickstart",
  title: "Quickstart",
  description: "Download StatGlass and approve the first launch.",
  group: "Getting started",
  markdown: "# Quickstart",
  markdownHref: "/docs/quickstart.md",
  markdownUrl: "https://example.com/docs/install.md",
};
const nav: DocNavGroup[] = [
  {
    title: "Getting started",
    items: [
      {
        id: "quickstart",
        href: "/docs/quickstart",
        label: "Quickstart",
        description: "",
      },
    ],
  },
];

it("frames the page with its title, group breadcrumb, description and body", () => {
  render(
    <DocsShell nav={nav} page={page} headings={[]}>
      <p>Body text</p>
    </DocsShell>
  );
  expect(
    screen.getAllByRole("heading", { level: 1 }).map((h) => h.textContent)
  ).toEqual(["Quickstart"]);
  expect(
    screen.getByRole("navigation", { name: "Breadcrumb" })
  ).toHaveTextContent("DocsGetting started");
  expect(
    screen.getByText("Download StatGlass and approve the first launch.")
  ).toBeVisible();
  expect(screen.getByText("Body text")).toBeVisible();
  expect(screen.queryByRole("region", { name: "Start here" })).toBeNull();
});

it("shows featured pages as cards under Start here", () => {
  render(
    <DocsShell
      nav={nav}
      page={{ ...page, id: "index", title: "StatGlass documentation" }}
      headings={[]}
      featured={[
        {
          id: "agents",
          href: "/docs/agents",
          label: "With your agent",
          description: "x",
        },
      ]}
    >
      <p>Home</p>
    </DocsShell>
  );
  const start = screen.getByRole("region", { name: "Start here" });
  expect(within(start).getByRole("link")).toHaveAttribute(
    "href",
    "/docs/agents"
  );
});
