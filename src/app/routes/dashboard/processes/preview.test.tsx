import { screen, waitFor, within } from "@testing-library/react";
import { renderWithProviders } from "@tests/test-utils";
import ProcessesRoute from "./route";

describe("Processes route column parameter", () => {
  it.each([
    ["cpu", "CPU", /% CPU/],
    ["memory", "Memory", "Memory"],
    ["energy", "Energy", "Energy"],
    ["disk", "Disk", "Disk total"],
    ["network", "Network", "Net total"],
    ["gpu", "GPU", /% GPU/],
    ["bogus", "CPU", /% CPU/],
    ["toString", "CPU", /% CPU/],
  ])("opens columns=%s with its default sort", async (value, label, header) => {
    renderWithProviders(<ProcessesRoute />, {
      route: `/dashboard/processes?columns=${value}`,
    });
    await screen.findByRole("cell", { name: /Xcode/ });
    expect(screen.getByRole("radio", { name: label })).toHaveAttribute(
      "aria-checked",
      "true"
    );
    expect(screen.getByRole("columnheader", { name: header })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    if (value === "gpu") {
      await waitFor(() =>
        expect(
          within(screen.getByRole("table", { name: "Processes" })).getAllByRole(
            "row"
          )[1]
        ).toHaveTextContent("WindowServer")
      );
    }
  });

  it("falls back to CPU when the requested set is not offered", async () => {
    renderWithProviders(<ProcessesRoute />, {
      route: "/dashboard/processes?columns=gpu",
      transportOptions: { scenarios: ["no-process-gpu"] },
    });
    await screen.findByRole("cell", { name: /Xcode/ });
    expect(screen.getByRole("radio", { name: "CPU" })).toHaveAttribute(
      "aria-checked",
      "true"
    );
    expect(screen.queryByRole("radio", { name: "GPU" })).toBeNull();
    expect(screen.getByRole("columnheader", { name: /% CPU/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
  });
});
