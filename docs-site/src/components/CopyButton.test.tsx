import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { CopyButton } from "./CopyButton";

it("renders hidden, so pages without scripts never show a dead button", () => {
  const { container } = render(<CopyButton id="target" />);
  const button = container.querySelector("button[data-copy-command]");
  expect(button).toHaveAttribute("hidden");
  expect(button).toHaveAttribute("type", "button");
  expect(button).toHaveAttribute("aria-controls", "target");
  expect(button).toHaveAttribute("aria-label", "Copy");
  expect(container.querySelector("[data-copy-label]")).toHaveTextContent(
    "Copy"
  );
});

it("takes its label from the label prop", () => {
  const { container } = render(<CopyButton id="page" label="Copy page" />);
  const button = container.querySelector("button");
  expect(button).toHaveAttribute("aria-label", "Copy page");
  expect(container.querySelector("[data-copy-label]")).toHaveTextContent(
    "Copy page"
  );
});

it("has a polite status region beside the button for the outcome", () => {
  const { container } = render(<CopyButton id="target" />);
  const status = container.querySelector("[data-copy-result]");
  expect(status).toHaveAttribute("role", "status");
  expect(status).toHaveAttribute("aria-live", "polite");
  expect(status).toBeEmptyDOMElement();
  expect(status?.parentElement).toBe(
    container.querySelector("button")?.parentElement
  );
});

it("hides its icon from assistive technology", () => {
  const { container } = render(<CopyButton id="target" />);
  expect(container.querySelector("button svg")).toHaveAttribute(
    "aria-hidden",
    "true"
  );
  expect(screen.queryByRole("button")).toBeNull(); // hidden buttons are not exposed
});
