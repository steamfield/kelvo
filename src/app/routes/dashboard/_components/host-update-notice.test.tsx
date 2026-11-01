import { createMockTransport } from "@core/mock-transport";
import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithProviders } from "../../../../../tests/test-utils";
import { HostUpdateNotice } from "./host-update-notice";

describe("host update notice", () => {
  it("counts hosts behind this Mac and stays dismissed for that version", async () => {
    const transport = createMockTransport({ scenarios: ["hosts-outdated"] });
    const { user } = renderWithProviders(<HostUpdateNotice />, {
      transport,
    });
    expect(
      await screen.findByText("4 hosts are running an older Kelvo.")
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("region", { name: "Host updates" })
      ).not.toBeInTheDocument()
    );
    expect(await transport.hostUpdateNoticeDismissed()).toEqual({
      status: "ok",
      data: "hosts:0.1.0",
    });
  });

  it("can't be dismissed while a host can't connect", async () => {
    const transport = createMockTransport({ scenarios: ["host-incompatible"] });
    renderWithProviders(<HostUpdateNotice />, { transport });
    expect(
      await screen.findByText("1 host can’t connect until it’s updated.")
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Dismiss" })
    ).not.toBeInTheDocument();
  });

  it("asks to update this Mac first when its own release is waiting", async () => {
    const transport = createMockTransport({
      scenarios: ["hosts-outdated"],
      updateStatus: { kind: "available", version: "0.4.0", notes: "" },
    });
    renderWithProviders(<HostUpdateNotice />, { transport });
    expect(
      await screen.findByText(
        "Kelvo 0.4.0 is available. Update this Mac first, then your hosts."
      )
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open Settings" })
    ).toBeInTheDocument();
  });

  it("waits for this Mac's restart while its update installs", async () => {
    const transport = createMockTransport({
      scenarios: ["hosts-outdated"],
      updateStatus: { kind: "installing", version: "0.4.0" },
    });
    renderWithProviders(<HostUpdateNotice />, { transport });
    expect(
      await screen.findByText(
        "Kelvo 0.4.0 is installing on this Mac. Update your hosts after it restarts."
      )
    ).toBeInTheDocument();
  });

  it("says nothing when hosts are current", async () => {
    const transport = createMockTransport({ scenarios: ["multi-host"] });
    renderWithProviders(<HostUpdateNotice />, { transport });
    await waitFor(async () =>
      expect(await transport.listRemoteHosts()).toHaveLength(1)
    );
    expect(
      screen.queryByRole("region", { name: "Host updates" })
    ).not.toBeInTheDocument();
  });
});
