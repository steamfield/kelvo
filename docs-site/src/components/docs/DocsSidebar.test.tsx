import { render, within } from "@testing-library/react";
import { expect, it } from "vitest";
import type { DocNavGroup } from "../../lib/docs";
import { DocsSidebar } from "./DocsSidebar";

const nav: DocNavGroup[] = [
  {
    title: "Getting started",
    items: [
      { id: "index", href: "/docs", label: "Introduction", description: "" },
      {
        id: "quickstart",
        href: "/docs/quickstart",
        label: "Quickstart",
        description: "",
      },
    ],
  },
  {
    title: "Metrics",
    items: [
      {
        id: "reference/commands",
        href: "/docs/reference/commands",
        label: "CLI commands",
        description: "",
      },
    ],
  },
];

// The phone disclosure and the desktop column hold the same navigation; CSS shows one of them.
it.each([
  ["the phone menu", "[data-docs-menu]"],
  ["the desktop column", "[data-docs-sidebar]"],
])(
  "%s lists each group's pages under its title and marks only the current page",
  (_name, selector) => {
    const { container } = render(
      <DocsSidebar nav={nav} current="quickstart" />
    );
    const copy = container.querySelector<HTMLElement>(selector);
    if (!copy) throw new Error(`no ${selector}`);
    const navigation = within(copy).getByRole("navigation", {
      name: "Documentation",
    });
    expect(
      within(navigation).getByRole("list", { name: "Getting started" })
    ).toHaveTextContent("IntroductionQuickstart");
    expect(
      within(navigation).getByRole("list", { name: "Metrics" })
    ).toHaveTextContent("CLI commands");
    const current = within(navigation)
      .getAllByRole("link")
      .filter((link) => link.hasAttribute("aria-current"));
    expect(
      current.map((link) => [
        link.textContent,
        link.getAttribute("aria-current"),
      ])
    ).toEqual([["Quickstart", "page"]]);
  }
);

it("puts the phone menu behind a Docs menu disclosure that needs no script", () => {
  const { container } = render(<DocsSidebar nav={nav} current="index" />);
  expect(
    container.querySelector("[data-docs-menu] > summary")
  ).toHaveTextContent("Docs menu");
});
