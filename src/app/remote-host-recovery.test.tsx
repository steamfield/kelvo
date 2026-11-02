import { createMockTransport } from "@core/mock-transport";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { App } from "./app";

it("keeps a never-connected host usable and offers a connection retry", async () => {
  const transport = createMockTransport({
    scenarios: ["host-unreachable"],
    historyRows: 0,
  });
  const view = render(
    <App transport={transport} router={{ initialEntry: "/dashboard/hosts" }} />
  );
  const heading = await screen.findByRole("heading", {
    name: "unreachable",
  });
  const row = heading.closest("div.flex-wrap");
  if (!row) throw new Error("missing remote host row");
  fireEvent.click(
    within(row as HTMLElement).getByRole("button", { name: "View metrics" })
  );
  await screen.findByRole("heading", { name: "Overview" });
  await screen.findByRole("heading", { name: "Can’t connect to unreachable" });
  expect(screen.getByRole("navigation")).toBeInTheDocument();
  expect(screen.getByText("Connecting", { exact: true })).toBeVisible();
  expect(screen.queryByText(/Sampling every 0s/)).not.toBeInTheDocument();
  expect(screen.queryByText("This view hit a snag")).not.toBeInTheDocument();
  fireEvent.click(screen.getByText("Technical details", { exact: true }));
  expect(screen.getByText(/SSH command failed \(127\)/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Retry connection" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Retry connection" })
    ).toBeEnabled()
  );
  expect(
    transport.calls.some((call) => call.command === "retry_remote_host")
  ).toBe(true);
  fireEvent.click(screen.getByRole("link", { name: "CPU" }));
  await screen.findByRole("heading", { name: "CPU" });
  expect(screen.queryByText("This view hit a snag")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("link", { name: "Open Hosts" }));
  await screen.findByRole("heading", { name: "Hosts" });
  view.unmount();
  transport.dispose();
});
