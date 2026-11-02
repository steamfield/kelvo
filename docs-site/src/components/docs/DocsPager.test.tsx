import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DocsPager } from "./DocsPager";

const page = (id: string, label: string) => ({
  id,
  href: `/docs/${id}`,
  label,
  description: "",
});

it("links the previous and next pages by name", () => {
  render(
    <DocsPager
      previous={page("index", "Introduction")}
      next={page("agents", "Agents")}
    />
  );
  const links = screen.getAllByRole("link");
  expect(
    links.map((link) => [link.textContent, link.getAttribute("href")])
  ).toEqual([
    ["PreviousIntroduction", "/docs/index"],
    ["NextAgents", "/docs/agents"],
  ]);
});

it("shows only the direction that exists at either end", () => {
  render(<DocsPager next={page("quickstart", "Quickstart")} />);
  expect(screen.getAllByRole("link").map((link) => link.textContent)).toEqual([
    "NextQuickstart",
  ]);
});

it("is left out when there is no other page", () => {
  const { container } = render(<DocsPager />);
  expect(container).toBeEmptyDOMElement();
});
