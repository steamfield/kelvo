import { createMockTransport } from "@core/mock-transport";
import { fireEvent, render, screen } from "@testing-library/react";
import { createTestQueryClient } from "@tests/test-utils";
import { createMemoryRouter } from "react-router";
import { LocalHost } from "~/app";
import { QueryProvider } from "~/lib/query";
import { TransportProvider } from "~/lib/transport-context";
import { appRoutes } from "~/router";
import { SettingsProvider } from "~/stores/settings-store";

function mountFailure() {
  let broken = true;
  function BrokenView() {
    if (broken)
      throw new RangeError(
        "Array length must be a positive integer of safe magnitude."
      );
    return <h1>Recovered view</h1>;
  }
  const routes = appRoutes({});
  const pages = routes[0]?.children?.find(
    (route) => route.path === "/dashboard"
  )?.children?.[0]?.children;
  if (!pages) throw new Error("missing dashboard routes");
  pages.push({ path: "broken", element: <BrokenView /> });
  const router = createMemoryRouter(routes, {
    initialEntries: ["/dashboard/broken"],
  });
  const transport = createMockTransport({ historyRows: 0 });
  const view = render(
    <TransportProvider transport={transport}>
      <QueryProvider client={createTestQueryClient()}>
        <SettingsProvider>
          <LocalHost router={router} />
        </SettingsProvider>
      </QueryProvider>
    </TransportProvider>
  );
  return {
    ...view,
    transport,
    recover: () => {
      broken = false;
    },
  };
}

describe("a dashboard render failure", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => vi.restoreAllMocks());

  it("keeps navigation, hides technical details initially, and retries the failed view", async () => {
    const view = mountFailure();
    await screen.findByRole("heading", { name: "This view hit a snag" });
    expect(screen.getByRole("navigation")).toBeInTheDocument();
    const details = screen.getByText("Technical details").closest("details");
    expect(details?.open).toBe(false);
    fireEvent.click(screen.getByText("Technical details"));
    expect(screen.getByText(/Array length must be/)).toBeVisible();
    expect(screen.getByText("View: /dashboard/broken")).toBeVisible();
    view.recover();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await screen.findByRole("heading", { name: "Recovered view" });
    view.unmount();
    view.transport.dispose();
  });

  it("can leave the failed view for Hosts without reloading the app", async () => {
    const view = mountFailure();
    await screen.findByRole("heading", { name: "This view hit a snag" });
    fireEvent.click(screen.getByRole("link", { name: "Open Hosts" }));
    await screen.findByRole("heading", { name: "Hosts" });
    expect(screen.queryByText("This view hit a snag")).not.toBeInTheDocument();
    view.unmount();
    view.transport.dispose();
  });
});
