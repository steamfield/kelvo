//! Interaction steps for the coalition benchmark (`plan/rfc-memory-footprint.md`, phase 1).
//!
//! `STATGLASS_BENCH_ACTIONS` is a comma-separated list the bench runs in order, starting
//! [`super::SETTLE`] after the scenario's window opens, so a run can measure what the
//! dashboard costs while it is used rather than only at rest:
//!
//! - `wait=<seconds>`: sleep.
//! - `window=<range>`: set the chart window through the same settings path the range
//!   control uses. `5m`, `15m`, `30m` and `1h` are the presets; `<N>h` and `<N>d` are
//!   rolling windows.
//! - `zoom`: on the Timeline, select the last two buckets with the keyboard (focus the
//!   cursor, Shift+ArrowLeft twice, Enter), which zooms through the animated preview.
//! - `back`: the Timeline's Back button.
//! - `live`: the Timeline's Live button.
//! - `hide`: close the dashboard as its close button does.
//! - `open=<route>`: open (or show) the dashboard at `<route>` (`/dashboard/cpu`).
//! - `popover`: show the popover as a tray click does; `popover-hide`: hide it as an
//!   outside click does. With `hide`, these trace what stays resident after a window
//!   closes.
//!
//! Each step is logged ("bench action"); `zoom` and `back` also log whether the chart
//! window changed, so a run where the page ignored them is visible in the log.

use std::time::Duration;

use statglass_schema::settings::ChartWindow;
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::windows::DASHBOARD;

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
/// How long a page-driven step gets to commit its range change before it is checked.
const SETTLE_STEP: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Wait(Duration),
    Window(ChartWindow),
    Zoom,
    Back,
    Live,
    Hide,
    Open(String),
    Popover,
    PopoverHide,
}

/// Parses the `STATGLASS_BENCH_ACTIONS` list; the first bad step is the error.
pub fn parse(list: &str) -> Result<Vec<Action>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|step| match step.split_once('=') {
            Some(("wait", secs)) => secs
                .parse::<u64>()
                .map(|s| Action::Wait(Duration::from_secs(s)))
                .map_err(|_| format!("wait takes whole seconds: {step}")),
            Some(("window", range)) => chart_window(range)
                .map(Action::Window)
                .ok_or_else(|| format!("unknown range: {step}")),
            None if step == "zoom" => Ok(Action::Zoom),
            None if step == "back" => Ok(Action::Back),
            None if step == "live" => Ok(Action::Live),
            Some(("open", route)) if route.starts_with("/dashboard") => {
                Ok(Action::Open(route.to_owned()))
            }
            None if step == "hide" => Ok(Action::Hide),
            None if step == "popover" => Ok(Action::Popover),
            None if step == "popover-hide" => Ok(Action::PopoverHide),
            _ => Err(format!("unknown step: {step}")),
        })
        .collect()
}

fn chart_window(range: &str) -> Option<ChartWindow> {
    if let Ok(preset) = serde_json::from_value(serde_json::Value::String(range.to_owned())) {
        return Some(preset);
    }
    let (n, unit) = range.split_at(range.len().checked_sub(1)?);
    let ms = match unit {
        "h" => HOUR_MS,
        "d" => DAY_MS,
        _ => return None,
    };
    let window = ChartWindow::Relative(n.parse::<i64>().ok()?.checked_mul(ms)?);
    window.valid().then_some(window)
}

/// The steps this launch was asked for; a list that does not parse runs nothing.
pub fn from_env() -> Vec<Action> {
    let Ok(list) = std::env::var("STATGLASS_BENCH_ACTIONS") else {
        return Vec::new();
    };
    parse(&list).unwrap_or_else(|e| {
        tracing::warn!("STATGLASS_BENCH_ACTIONS: {e}; running no actions");
        Vec::new()
    })
}

/// Runs `actions` in order on the calling thread (never the main thread: steps sleep).
pub fn run(app: &AppHandle, actions: &[Action]) {
    for action in actions {
        tracing::info!(?action, "bench action");
        match action {
            Action::Wait(d) => std::thread::sleep(*d),
            Action::Window(w) => set_window(app, *w),
            Action::Zoom => checked(app, action, ZOOM_JS),
            Action::Back => checked(app, action, &click_js("Back")),
            Action::Live => eval(app, &click_js("Live")),
            Action::Hide => hide(app),
            Action::Open(route) => on_main(app, {
                let route = route.clone();
                move |app| {
                    if let Err(e) = crate::windows::open_dashboard(app, Some(&route)) {
                        tracing::warn!("bench action: opening {route}: {e:?}");
                    }
                }
            }),
            Action::Popover => on_main(app, popover_show),
            Action::PopoverHide => on_main(app, crate::popover::hide),
        }
    }
    tracing::info!("bench actions done");
}

fn set_window(app: &AppHandle, window: ChartWindow) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let patch =
        serde_json::from_value(serde_json::json!({ "general": { "chart_window": window } }));
    match patch.map_err(|e| e.to_string()) {
        Ok(patch) => {
            if let Err(e) = state.update_settings(app, &patch) {
                tracing::warn!("bench action: setting the chart window: {e:?}");
            }
        }
        Err(e) => tracing::warn!("bench action: chart window patch: {e}"),
    }
}

fn current_window(app: &AppHandle) -> Option<ChartWindow> {
    app.try_state::<AppState>()
        .map(|s| s.settings.settings().general.chart_window)
}

/// Runs a page step and logs whether it moved the chart window.
fn checked(app: &AppHandle, action: &Action, js: &str) {
    let before = current_window(app);
    eval(app, js);
    std::thread::sleep(SETTLE_STEP);
    if current_window(app) == before {
        tracing::warn!(?action, "bench action had no effect on the chart window");
    } else {
        tracing::info!(?action, "bench action applied");
    }
}

fn eval(app: &AppHandle, js: &str) {
    match app.get_webview_window(DASHBOARD) {
        Some(w) => {
            if let Err(e) = w.eval(js) {
                tracing::warn!("bench action: eval: {e}");
            }
        }
        None => tracing::warn!("bench action: no dashboard window"),
    }
}

fn hide(app: &AppHandle) {
    let Some(w) = app.get_webview_window(DASHBOARD) else {
        tracing::warn!("bench action: no dashboard window to hide");
        return;
    };
    let window = w.as_ref().window();
    if let Err(e) = app.run_on_main_thread(move || crate::windows::hide_dashboard(&window)) {
        tracing::warn!("bench action: hide: {e}");
    }
}

fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || f(&handle)) {
        tracing::warn!("bench action: {e}");
    }
}

#[cfg(target_os = "macos")]
fn popover_show(app: &AppHandle) {
    crate::popover::toggle(app, std::time::Instant::now(), crate::tray::TRAY_ID);
}

#[cfg(not(target_os = "macos"))]
fn popover_show(_app: &AppHandle) {}

/// Focus the Timeline cursor, extend a selection two buckets left, commit it. The gaps
/// let React commit each keydown's state before the next one reads it.
const ZOOM_JS: &str = "(() => { \
    const s = document.querySelector('[aria-label=\"Timeline cursor\"]'); \
    if (!s) { console.warn('[bench] no Timeline cursor'); return; } \
    s.focus(); \
    const key = (key, shiftKey) => s.dispatchEvent(new KeyboardEvent('keydown', { key, shiftKey, bubbles: true })); \
    setTimeout(() => key('ArrowLeft', true), 100); \
    setTimeout(() => key('ArrowLeft', true), 200); \
    setTimeout(() => key('Enter', false), 400); \
})()";

fn click_js(label: &str) -> String {
    format!(
        "(() => {{ const b = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === {label:?}); \
         if (b) b.click(); else console.warn('[bench] no {label} button'); }})()"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_step() {
        assert_eq!(
            parse(
                "window=30d, zoom,wait=6,back,live,window=1h,window=24h,hide,open=/dashboard/cpu,popover,popover-hide"
            ),
            Ok(vec![
                Action::Window(ChartWindow::Relative(30 * DAY_MS)),
                Action::Zoom,
                Action::Wait(Duration::from_secs(6)),
                Action::Back,
                Action::Live,
                Action::Window(ChartWindow::H1),
                Action::Window(ChartWindow::Relative(24 * HOUR_MS)),
                Action::Hide,
                Action::Open("/dashboard/cpu".into()),
                Action::Popover,
                Action::PopoverHide,
            ])
        );
        assert_eq!(parse(""), Ok(vec![]));
    }

    #[test]
    fn rejects_bad_steps_and_ranges() {
        for bad in [
            "zoom,jump",
            "wait=1.5",
            "window=2w",
            "window=91d",
            "window=h",
            "open=/settings",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn click_script_quotes_the_label() {
        assert!(click_js("Back").contains("=== \"Back\""));
    }
}
