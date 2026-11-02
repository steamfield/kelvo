import { afterEach, describe, expect, it, vi } from "vitest";
import { enhanceCopyButtons } from "@steamfield/ui/copy";

const COPIED_MS = 2000;

function setup() {
  document.body.innerHTML = `
    <pre id="code-1">bun install</pre>
    <div>
      <button data-copy-command aria-controls="code-1" aria-label="Copy code" hidden>
        <span data-copy-label>Copy</span>
      </button>
      <span data-copy-result></span>
    </div>
    <button data-copy-command aria-controls="missing" hidden>Copy</button>`;
  enhanceCopyButtons(document);
  const [button, orphan] = document.querySelectorAll("button");
  const result = document.querySelector("[data-copy-result]");
  return { button: button!, orphan: orphan!, result: result! };
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("enhanceCopyButtons", () => {
  it("shows buttons whose target exists and leaves the rest hidden", () => {
    const { button, orphan } = setup();
    expect(button.hidden).toBe(false);
    expect(orphan.hidden).toBe(true);
  });

  it("copies the target's text and says so", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const { button, result } = setup();

    button.click();

    await vi.waitFor(() =>
      expect(result.textContent).toBe("Copied to clipboard.")
    );
    expect(writeText).toHaveBeenCalledWith("bun install");
    expect(button.textContent?.trim()).toBe("Copied");
  });

  it("goes back to its label after a moment, ready to copy again", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("navigator", {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    const { button, result } = setup();

    button.click();
    await vi.waitFor(() => expect(button.textContent?.trim()).toBe("Copied"));
    vi.advanceTimersByTime(COPIED_MS);

    expect(button.textContent?.trim()).toBe("Copy");
    expect(result.textContent).toBe("");
  });

  it("selects the text when the clipboard is blocked", async () => {
    vi.stubGlobal("navigator", {
      clipboard: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    const { button, result } = setup();

    button.click();

    await vi.waitFor(() =>
      expect(result.textContent).toBe(
        "Copy failed. The code is selected; press Cmd+C or Ctrl+C."
      )
    );
    expect(document.getSelection()?.toString()).toBe("bun install");
  });
});

describe("when used again", () => {
  it("puts back the idle label after a second click, not the copied one", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("navigator", {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    const { button } = setup();

    button.click();
    await vi.waitFor(() => expect(button.textContent?.trim()).toBe("Copied"));
    button.click();
    await vi.advanceTimersByTimeAsync(COPIED_MS);

    expect(button.textContent?.trim()).toBe("Copy");
  });

  it("shows the failure message to everyone, marked as failed", async () => {
    vi.stubGlobal("navigator", {
      clipboard: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    const { button, result } = setup();

    button.click();

    await vi.waitFor(() => expect(result.textContent).toMatch(/^Copy failed/));
    expect(result).toHaveAttribute("data-state", "failed");
    expect(button).toHaveAttribute("data-state", "failed");
  });
});
