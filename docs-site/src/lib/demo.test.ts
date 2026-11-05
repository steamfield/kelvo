import { describe, expect, it } from "vitest";
import { dashboardDemoUrl, demoPainted, readDemoMessage } from "./demo";

describe("dashboardDemoUrl", () => {
  it("keeps outer parameters separate from an encoded route query", () => {
    const route = "/dashboard/disk?view=space&vol=/&tab=largest";
    const url = new URL(
      dashboardDemoUrl(route, {
        scenario: "multi-host",
        space_scan: "/",
        host: "remote host",
        chart_window: "24h",
      }),
      "https://statglass.steamfield.io"
    );
    expect(Object.fromEntries(url.searchParams)).toEqual({
      window: "dashboard",
      scenario: "multi-host",
      space_scan: "/",
      host: "remote host",
      chart_window: "24h",
      route,
    });
    expect(url.searchParams.has("tab")).toBe(false);
    expect(
      new URL(
        dashboardDemoUrl("/dashboard/processes?columns=gpu"),
        url
      ).searchParams.get("route")
    ).toBe("/dashboard/processes?columns=gpu");
  });
  it("opens the dashboard at a dashboard route and ignores anything else", () => {
    expect(dashboardDemoUrl("/dashboard/cpu")).toBe(
      "/demo/index.html?window=dashboard&route=%2Fdashboard%2Fcpu"
    );
    expect(dashboardDemoUrl(null)).toBe("/demo/index.html?window=dashboard");
    expect(dashboardDemoUrl("https://evil.example/")).toBe(
      "/demo/index.html?window=dashboard"
    );
  });
});

describe("readDemoMessage", () => {
  it("reads the size and open-dashboard messages the app's bridge posts", () => {
    expect(
      readDemoMessage({ type: "statglass-demo:size", height: 512 })
    ).toEqual({ type: "size", height: 512 });
    expect(
      readDemoMessage({
        type: "statglass-demo:open-dashboard",
        route: "/dashboard/gpu",
      })
    ).toEqual({ type: "open-dashboard", route: "/dashboard/gpu" });
    expect(
      readDemoMessage({ type: "statglass-demo:open-dashboard", route: null })
    ).toEqual({ type: "open-dashboard", route: null });
  });

  it("rejects other messages and unusable sizes", () => {
    for (const data of [
      null,
      "statglass-demo:size",
      { type: "other" },
      { type: "statglass-demo:size", height: Number.NaN },
      { type: "statglass-demo:size", height: -1 },
      { type: "statglass-demo:size", height: "512" },
    ]) {
      expect(readDemoMessage(data)).toBeNull();
    }
  });
});

describe("demoPainted", () => {
  function doc(html: string, dark: boolean) {
    const d = document.implementation.createHTMLDocument("demo");
    d.body.innerHTML = html;
    d.documentElement.classList.toggle("dark", dark);
    return d;
  }

  it("waits for the app to render, in the page's theme", () => {
    expect(demoPainted(doc('<div id="root"></div>', true), true)).toBe(false);
    expect(
      demoPainted(doc('<div id="root"><main></main></div>', false), true)
    ).toBe(false);
    expect(
      demoPainted(doc('<div id="root"><main></main></div>', true), true)
    ).toBe(true);
    expect(
      demoPainted(doc('<div id="root"><main></main></div>', false), false)
    ).toBe(true);
  });
});
