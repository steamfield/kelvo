import type {
  CliLinkState,
  CliLinkStatus,
  UpdateStatus,
} from "@core/generated/bindings";
import type { MockCommands } from "./commands";
import type { MockTransport, MockTransportOptions } from "./contracts";
import type { MockEmitters } from "./emitters";
import type { MockPower } from "./power";
import { ok } from "./protocol";
import type { MockRing } from "./ring";
import type { MockSettings } from "./settings";

const LINK_PATH = "/Users/you/.local/bin/statglass";
const CLI_TARGET = "/Applications/StatGlass.app/Contents/MacOS/statglass-cli";
/** A plausible `cli_link_status` for each state (dev server `?cli_link=`). */
export function cliLinkFixture(state: CliLinkState): CliLinkStatus {
  const points_to = {
    missing: null,
    current: CLI_TARGET,
    stale: "/Users/you/Downloads/StatGlass.app/Contents/MacOS/statglass-cli",
    foreign: "/opt/homebrew/bin/statglass",
    blocked: null,
  }[state];
  const reason =
    state === "blocked"
      ? `${LINK_PATH} is not a link. Move it to install.`
      : null;
  return {
    link_path: LINK_PATH,
    target: CLI_TARGET,
    state,
    points_to,
    reason,
    on_path: false,
  };
}
export class MockShell {
  private updateStatus: UpdateStatus;
  private cliLink: CliLinkStatus;
  constructor(
    private readonly ring: MockRing,
    private readonly power: MockPower,
    private readonly settings: MockSettings,
    private readonly commands: MockCommands,
    private readonly emitters: MockEmitters,
    private readonly options: MockTransportOptions
  ) {
    this.updateStatus = options.updateStatus ?? { kind: "idle" };
    this.cliLink = options.cliLink ?? cliLinkFixture("missing");
  }
  readonly requestNavigate: MockTransport["requestNavigate"] = (route) => {
    this.emitters.navigate.emit({ route });
  };
  readonly reportPopoverContentHeight: MockTransport["reportPopoverContentHeight"] =
    async (height) => {
      if (Number.isFinite(height) && height > 0 && height <= 100_000) {
        this.commands.record("report_popover_content_height", height);
        this.options.embed?.popoverContentHeight(height);
      }
    };
  readonly openDashboard: MockTransport["openDashboard"] = async (route) => {
    this.commands.record("open_dashboard", route);
    this.options.embed?.openDashboard(route);
    return ok(null);
  };
  readonly getEdition: MockTransport["getEdition"] = async () => {
    this.commands.record("get_edition");
    return {
      process_signal: !this.ring.flags.appstore,
      disk_space: !this.ring.flags.appstore,
    };
  };
  readonly cliLinkStatus: MockTransport["cliLinkStatus"] = async () => {
    this.commands.record("cli_link_status");
    return ok(this.cliLink);
  };
  /** As `cli_install_link`: `blocked` stays; any symlink is re-pointed at the target. */
  readonly cliInstallLink: MockTransport["cliInstallLink"] = async () => {
    this.commands.record("cli_install_link");
    if (this.cliLink.target === null)
      return {
        status: "error",
        error: {
          kind: "invalid_argument",
          message: "This build has no bundled command-line tool",
        },
      };
    if (this.cliLink.state !== "blocked")
      this.cliLink = {
        ...this.cliLink,
        state: "current",
        points_to: this.cliLink.target,
        reason: null,
      };
    return ok(this.cliLink);
  };
  readonly getUpdateStatus: MockTransport["getUpdateStatus"] = async () =>
    this.updateStatus;
  readonly onUpdateStatusChanged: MockTransport["onUpdateStatusChanged"] = (
    cb
  ) => this.emitters.updates.on(cb);
  readonly checkForUpdates: MockTransport["checkForUpdates"] = async () => {
    this.commands.record("check_for_updates");
    this.updateStatus = this.options.updateStatus ?? { kind: "up_to_date" };
    this.emitters.updates.emit(this.updateStatus);
    return ok(this.updateStatus);
  };
  readonly installUpdate: MockTransport["installUpdate"] = async (version) => {
    this.commands.record("install_update", version);
    if (this.options.updateFails) {
      this.updateStatus = {
        kind: "failed",
        message: "The update signature could not be verified.",
      };
      this.emitters.updates.emit(this.updateStatus);
      return {
        status: "error",
        error: { kind: "update", message: this.updateStatus.message },
      };
    }
    this.updateStatus = { kind: "ready", version };
    this.emitters.updates.emit(this.updateStatus);
    return ok(null);
  };
  readonly getWindowAppearance: MockTransport["getWindowAppearance"] =
    async () => {
      this.commands.record("get_window_appearance");
      return {
        performance: this.power.status.performance,
        reduce_transparency: false,
        theme: this.settings.snapshot.settings.general.appearance,
      };
    };
  readonly openUrl: MockTransport["openUrl"] = async (url) => {
    this.commands.record("open_url", url);
  };
  readonly closeWindow: MockTransport["closeWindow"] = async () => {
    this.commands.record("close_window");
  };

  get methods() {
    return {
      requestNavigate: this.requestNavigate,
      reportPopoverContentHeight: this.reportPopoverContentHeight,
      openDashboard: this.openDashboard,
      getEdition: this.getEdition,
      cliLinkStatus: this.cliLinkStatus,
      cliInstallLink: this.cliInstallLink,
      checkForUpdates: this.checkForUpdates,
      getUpdateStatus: this.getUpdateStatus,
      installUpdate: this.installUpdate,
      onUpdateStatusChanged: this.onUpdateStatusChanged,
      getWindowAppearance: this.getWindowAppearance,
      openUrl: this.openUrl,
      closeWindow: this.closeWindow,
    };
  }
}
