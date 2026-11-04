import {
  type TrayStyle,
  TrayPreview,
  trayLayout,
  trayStyleMenuBar,
} from "@statglass/widgets";
import { UNITS } from "@/lib/tray-sample";
import { useLiveTray } from "@/lib/use-live-tray";

type MenuBar = ReturnType<typeof trayStyleMenuBar>;

/** The onboarding choices, plus the per-core strip from Settings. */
const STYLES: { name: string; detail: string; menuBar: MenuBar }[] = [
  ...(
    [
      ["combined", "Combined", "One item. The default, and the smallest."],
      ["values", "Values only", "Numbers with stacked labels."],
      [
        "graphs",
        "Graph per module",
        "Separate items you can reorder with ⌘‑drag.",
      ],
    ] as const satisfies readonly (readonly [TrayStyle, string, string])[]
  ).map(([style, name, detail]) => ({
    name,
    detail,
    menuBar: trayStyleMenuBar(style),
  })),
  {
    name: "Per-core strip",
    detail: "Every core's load, performance cores first.",
    menuBar: {
      ...trayStyleMenuBar("combined"),
      bars: { cpu: false, gpu: false, memory: false },
      readouts: {
        ...trayStyleMenuBar("combined").readouts,
        temperature: false,
      },
      items: { ...trayStyleMenuBar("combined").items, cpu: "cores" },
    },
  },
];

/** Each menu bar style, live on the same simulated readings. */
export function MenuBarStyles() {
  const { ref, readings } = useLiveTray();
  return (
    <dl ref={ref} className="divide-y divide-border border-y border-border">
      {STYLES.map(({ name, detail, menuBar }) => (
        <div
          key={name}
          data-motion-part
          className="grid items-center gap-x-6 gap-y-2 py-4 sm:grid-cols-[minmax(0,1fr)_auto]"
        >
          <div>
            <dt className="text-sm font-[590]">{name}</dt>
            <dd className="text-sm text-secondary-foreground">{detail}</dd>
          </div>
          <dd className="min-w-0">
            <div data-nosnippet="">
              <TrayPreview
                layout={trayLayout(menuBar, () => true, readings, UNITS)}
                className="sm:w-[240px]"
              />
            </div>
          </dd>
        </div>
      ))}
    </dl>
  );
}
