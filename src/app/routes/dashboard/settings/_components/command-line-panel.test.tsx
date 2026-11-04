import type { CliLinkStatus } from "@core/generated/bindings";
import { createMockTransport } from "@core/mock-transport";
import { screen, waitFor } from "@testing-library/react";
import { renderWithProviders } from "../../../../../../tests/test-utils";
import SettingsRoute from "../route";
import { CommandLinePanel } from "./command-line-panel";

const LINK: CliLinkStatus = {
  link_path: "/Users/riley/.local/bin/statglass",
  target: "/Applications/StatGlass.app/Contents/MacOS/statglass-cli",
  state: "missing",
  on_path: true,
};
/** The link row's label and sub. */
const linkRow = () =>
  screen.getByText("statglass command").parentElement as HTMLElement;
/** Waits for the link row to say `text` (the status loads after the first render). */
const expectLink = (text: string) =>
  waitFor(() => expect(linkRow()).toHaveTextContent(text));

describe("Command line settings (D-126)", () => {
  it("the access switch is on by default and writes a one-key patch", async () => {
    const { transport, user } = renderWithProviders(<SettingsRoute />);
    const toggle = await screen.findByRole("switch", {
      name: "Allow command-line access",
    });
    expect(toggle).toBeChecked();
    await user.click(toggle);
    await waitFor(() =>
      expect(transport.calls).toContainEqual({
        command: "update_settings",
        args: [{ cli_access: { enabled: false } }],
      })
    );
    await waitFor(() =>
      expect(
        screen.getByRole("switch", { name: "Allow command-line access" })
      ).not.toBeChecked()
    );
  });

  it("installs a missing link and then shows where it is", async () => {
    const { transport, user } = renderWithProviders(
      <CommandLinePanel enabled />,
      { transportOptions: { cliLink: LINK } }
    );
    await expectLink("Not installed.");
    await user.click(screen.getByRole("button", { name: "Install" }));
    await expectLink("Installed at ~/.local/bin/statglass.");
    expect(linkRow()).not.toHaveTextContent(/PATH/);
    expect(
      screen.queryByRole("button", { name: /Install|Repair/ })
    ).not.toBeInTheDocument();
    expect(
      transport.calls.filter((c) => c.command === "cli_install_link")
    ).toHaveLength(1);
  });

  it("suggests adding the directory to PATH only when the app's PATH lacks it", async () => {
    renderWithProviders(<CommandLinePanel enabled />, {
      transportOptions: {
        cliLink: { ...LINK, state: "current", on_path: false },
      },
    });
    await expectLink(
      "If your shell can't find statglass, add ~/.local/bin to PATH."
    );
  });

  it("repairs a link to an older copy", async () => {
    const { transport, user } = renderWithProviders(
      <CommandLinePanel enabled />,
      { transportOptions: { cliLink: { ...LINK, state: "stale" } } }
    );
    await expectLink(
      "~/.local/bin/statglass points to an older copy of StatGlass."
    );
    await user.click(screen.getByRole("button", { name: "Repair" }));
    await expectLink("Installed at");
    expect(transport.calls).toContainEqual({
      command: "cli_install_link",
      args: [],
    });
  });

  it("does not offer to replace a file that isn't a link", async () => {
    renderWithProviders(<CommandLinePanel enabled />, {
      transportOptions: { cliLink: { ...LINK, state: "blocked" } },
    });
    await expectLink(
      "~/.local/bin/statglass is a file, not a link. Move it to install."
    );
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
  });

  it("disables install in a build without a bundled CLI", async () => {
    renderWithProviders(<CommandLinePanel enabled />, {
      transportOptions: { cliLink: { ...LINK, target: null } },
    });
    await expectLink("This build has no bundled command-line tool.");
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
  });

  it("says why an install failed and leaves the button usable", async () => {
    const transport = createMockTransport({ cliLink: LINK });
    transport.cliInstallLink = async () => ({
      status: "error",
      error: { kind: "internal", message: "Permission denied" },
    });
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { user } = renderWithProviders(<CommandLinePanel enabled />, {
      transport,
    });
    await user.click(await screen.findByRole("button", { name: "Install" }));
    await expectLink("Couldn't install: Permission denied");
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled();
  });

  it("copies the skill install command", async () => {
    const { user } = renderWithProviders(<CommandLinePanel enabled />);
    const write = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue(undefined);
    await user.click(screen.getByRole("button", { name: "Copy command" }));
    expect(write).toHaveBeenCalledWith("statglass skill install");
    expect(
      await screen.findByRole("button", { name: "Copied" })
    ).toBeInTheDocument();
  });
});
