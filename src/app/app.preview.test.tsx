import { createMockTransport, type MockTransport } from "@core/mock-transport";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./app";

describe("browser preview initial host", () => {
  let transport: MockTransport;
  afterEach(() => {
    cleanup();
    transport?.dispose();
    window.history.replaceState(null, "", "/");
  });

  it.each([
    ["linux-host", "remote-linux", "linux-tailscale"],
    ["multi-host", "remote-studio", "Studio Mac"],
    ["linux-host", "unknown", "This Mac"],
    ["linux-host", undefined, "This Mac"],
  ] as const)(
    "selects %s host %s if it exists",
    async (scenario, host, selected) => {
      // The outer URL alone cannot select a host: main must pass initialHost.
      window.history.replaceState(null, "", "?host=remote-linux");
      transport = createMockTransport({
        scenarios: [scenario],
        autoTick: false,
      });
      render(
        <App
          transport={transport}
          {...(host ? { initialHost: host } : {})}
          router={{ initialEntry: "/dashboard/overview" }}
        />
      );
      expect(
        await screen.findByRole("combobox", { name: "Selected machine" })
      ).toHaveTextContent(selected);
    }
  );

  it("lets the reader switch away from the initial host", async () => {
    transport = createMockTransport({
      scenarios: ["linux-host"],
      autoTick: false,
    });
    render(
      <App
        transport={transport}
        initialHost="remote-linux"
        router={{ initialEntry: "/dashboard/overview" }}
      />
    );
    const user = userEvent.setup();
    await user.click(
      await screen.findByRole("combobox", { name: "Selected machine" })
    );
    await user.click(await screen.findByRole("option", { name: /This Mac/ }));
    expect(
      screen.getByRole("combobox", { name: "Selected machine" })
    ).toHaveTextContent("This Mac");
  });
});
