/**
 * Enhances `[data-copy-command]` buttons (code blocks, "Copy page"): each
 * names the element to copy with `aria-controls`, starts hidden, and has a
 * `[data-copy-result]` beside it for the outcome. When the clipboard is
 * blocked, the text is selected so the reader can copy it by hand.
 */
/** How long a button says "Copied" before it can be used again. */
export const COPIED_MS = 2000;

const SUBJECTS: Record<string, string> = {
  "Copy code": "code",
  "Copy page": "page",
};

export function enhanceCopyButtons(root: Document): void {
  for (const button of root.querySelectorAll<HTMLButtonElement>(
    "[data-copy-command]"
  )) {
    const targetId = button.getAttribute("aria-controls");
    if (!targetId || !root.getElementById(targetId)) continue;
    button.hidden = false;
    button.addEventListener("click", () => {
      void copy(root, button, targetId);
    });
  }
}

async function copy(root: Document, button: HTMLElement, targetId: string) {
  const target = root.getElementById(targetId);
  if (!target) return;
  const result = button.parentElement?.querySelector("[data-copy-result]");
  try {
    await navigator.clipboard.writeText(target.textContent ?? "");
    if (result) result.textContent = "Copied to clipboard.";
    const label = button.querySelector("[data-copy-label]");
    const idle = label?.textContent ?? "";
    if (label) label.textContent = "Copied";
    button.setAttribute("title", "Copied to clipboard");
    // Restore the idle state so the next copy gives the same feedback.
    setTimeout(() => {
      if (label) label.textContent = idle;
      if (result) result.textContent = "";
      button.removeAttribute("title");
    }, COPIED_MS);
  } catch {
    const disclosure = target.closest("details");
    if (disclosure) disclosure.open = true;
    const range = root.createRange();
    range.selectNodeContents(target);
    const selection = root.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    if (result) {
      const subject =
        SUBJECTS[button.getAttribute("aria-label") ?? ""] ?? "text";
      result.textContent = target.hidden
        ? `Copy failed. Open the ${subject} as Markdown and copy it from there.`
        : `Copy failed. The ${subject} is selected; press Cmd+C.`;
    }
  }
}
