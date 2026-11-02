import { TrayPreview, trayLayout, trayStyleMenuBar } from "@statglass/widgets";
import { UNITS } from "@/lib/tray-sample";
import { useLiveTray } from "@/lib/use-live-tray";

/**
 * The default menu bar item (three bars and the temperature) over simulated
 * readings, laid out by the app's own rules. It sits above the popover demo
 * the way the real item sits above the real popover.
 */
export function MenuBarItem() {
  const { ref, readings } = useLiveTray();
  const layout = trayLayout(
    trayStyleMenuBar("combined"),
    () => true,
    readings,
    UNITS
  );
  return (
    <div ref={ref}>
      <TrayPreview layout={layout} />
    </div>
  );
}
