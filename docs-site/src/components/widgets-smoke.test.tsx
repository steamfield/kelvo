import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModuleCard, RingGauge, StreamArea } from "@statglass/widgets";

// The site renders real app widgets. If the barrel, the aliases or a widget's
// props change in a way the site can't follow, this fails in `make test`.
describe("StatGlass widgets in docs-site", () => {
  it("renders a stream chart, a ring and a module card", () => {
    render(
      <ModuleCard accent="cpu" title="CPU" value="18" unit="%">
        <RingGauge
          fractions={[0.124, 0.056]}
          value="18%"
          label="Load"
          accent="cpu"
          size={72}
        />
        <StreamArea
          series={[{ key: "total", values: [10, 12, 14, 13, 18], step: 1 }]}
          tEndMs={1_760_000_000_000}
          intervalMs={1000}
          yMax={40}
          accent="cpu"
          height={52}
          ariaLabel="CPU, last 60 seconds"
        />
      </ModuleCard>
    );

    expect(screen.getByText("CPU")).toBeInTheDocument();
    expect(screen.getByText("Load")).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: "CPU, last 60 seconds" })
    ).toBeInTheDocument();
  });
});
