//! `STATGLASS_BENCH_WK_FEATURES=AcceleratedDrawingEnabled=1,...`: override WebKit
//! internal features on every open webview for one bench launch, then reload it, to
//! measure what a feature costs the WebKit helpers against `platform::webkit::DEFAULTS`.

use tauri::{AppHandle, Manager};

pub fn apply_from_env(app: &AppHandle) {
    let Ok(list) = std::env::var("STATGLASS_BENCH_WK_FEATURES") else {
        return;
    };
    let wanted: Vec<(String, bool)> = list
        .split(',')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim() == "1"))
        .collect();
    if wanted.is_empty() {
        return;
    }
    for (label, window) in app.webview_windows() {
        crate::platform::webkit::set_and_reload(window.as_ref(), wanted.clone());
        tracing::info!(%label, features = %list, "bench webkit features");
    }
}
