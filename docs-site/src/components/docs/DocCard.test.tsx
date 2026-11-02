import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { DocLink } from "@/lib/docs";
import { DocCardGrid } from "./DocCard";

const link = (id: string): DocLink => ({
  id,
  href: `/docs/${id}`,
  label: `Label ${id}`,
  description: `About ${id}`,
});

it("links each card to its page, named by its label and showing its description", () => {
  const ids = ["install", "remote/linux"];
  render(<DocCardGrid links={ids.map(link)} />);
  const cards = screen.getAllByRole("link");
  expect(cards.map((card) => card.getAttribute("href"))).toEqual(
    ids.map((id) => `/docs/${id}`)
  );
  expect(cards[1]).toHaveAccessibleName(
    "Label remote/linux About remote/linux"
  );
});

it("renders nothing without links", () => {
  const { container } = render(<DocCardGrid links={[]} />);
  expect(container).toBeEmptyDOMElement();
});
