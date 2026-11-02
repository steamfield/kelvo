/**
 * Public surface of the widget library for docs-site, which renders real
 * widgets on the marketing pages. The app keeps importing widgets by file.
 * Named re-exports of `./` files, plus the pure `@core` functions that
 * build a widget's props (the menu bar layout), so the site lays out the
 * menu bar with the app's rules. No `export *`, no sample data.
 */
export { type TrayStyle, trayStyleMenuBar } from "@core/settings-patch";
export {
  type TrayLayout,
  type TrayReadings,
  type TrayUnits,
  trayLayout,
} from "@core/tray-layout";
export {
  BatteryHistoryBars,
  type BatteryHistoryBarsProps,
} from "./battery-history-bars";
export { Card, type CardProps, LinkCard, type LinkCardProps } from "./card";
export { CardNotice, type CardNoticeProps } from "./card-notice";
export {
  ChartAnnotation,
  type ChartAnnotationProps,
} from "./chart-annotation";
export { CoreHeatmap, type CoreHeatmapProps } from "./core-heatmap";
export {
  CoreTiles,
  type CoreTilesCluster,
  type CoreTilesProps,
} from "./core-tiles";
export {
  GapBand,
  type GapBandProps,
  GapBands,
  type GapSpan,
} from "./gap-band";
export {
  HeatMatrix,
  type HeatMatrixProps,
  type HeatMatrixRow,
  HeatScaleLegend,
} from "./heat-matrix";
export { InitialChip, type InitialChipProps } from "./initial-chip";
export { InlineBar, type InlineBarProps } from "./inline-bar";
export {
  Legend,
  type LegendItem,
  type LegendProps,
  Swatch,
  type SwatchStep,
} from "./legend";
export type { Accent, Corner, RampStep } from "./lib/accent";
export { MeterTrack, type MeterTrackProps } from "./meter-track";
export {
  MetricCard,
  type MetricCardBody,
  type MetricCardProps,
} from "./metric-card";
export { MirrorBars, type MirrorBarsProps } from "./mirror-bars";
export { ModuleCard, type ModuleCardProps } from "./module-card";
export {
  ProcessList,
  type ProcessListProps,
  type ProcessListRow,
} from "./process-list";
export { ResidencyBar, type ResidencyBarProps } from "./residency-bar";
export { RingGauge, type RingGaugeProps, type RingSize } from "./ring-gauge";
export { RingStatCard, type RingStatCardProps } from "./ring-stat-card";
export {
  StackBar,
  type StackBarProps,
  type StackBarSegment,
} from "./stack-bar";
export {
  StatGrid,
  type StatGridItem,
  type StatGridProps,
} from "./stat-grid";
export {
  StatStrip,
  type StatStripItem,
  type StatStripProps,
} from "./stat-strip";
export { StatusPill, type StatusPillProps } from "./status-pill";
export { StreamArea, type StreamAreaProps } from "./stream-area";
export { TrayPreview, type TrayPreviewProps } from "./tray-preview";
export { WindowTicks, type WindowTicksProps } from "./window-ticks";
