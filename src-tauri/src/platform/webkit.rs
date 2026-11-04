//! WebKit internal features for StatGlass webviews (plan/rfc-memory-footprint.md).
//!
//! `AcceleratedDrawingEnabled` ("GraphicsLayer accelerated drawing") is off in the
//! popover. With it on, a webview that repaints every second (any live
//! number or chart) has Core Animation's Metal drawing keep 28 GPU allocations of 8 MB
//! resident in the WebKit GPU process, about 250 MB whatever the window's size or content.
//! WebKit only lets them go after a few seconds without a repaint, which a 1 Hz monitor
//! never has. The GPU process is shared, so the cost is paid once while any such window is
//! visible. Off, layers are drawn on the CPU: with the popover open the GPU process stays
//! near 16 MB (from 265) for 0.3 to 0.5 points more CPU. The dashboard keeps it on because
//! it repaints far more area each second: off, it saves the same 250 MB but costs about 6
//! points of CPU (9.8% to 15.7% on Overview). Canvas drawing has its own switch
//! (`CanvasUsesAcceleratedDrawing`) and stays accelerated.
//!
//! The features go on the `WKWebViewConfiguration` the webview is built with
//! ([`configure`]). Setting them on a live webview as its page starts loading races the
//! first paint: on some launches the first layers were already accelerated, and the pool
//! came back every 10 s while the popover was open.
//!
//! These are WebKit SPI (`+[WKPreferences _features]`, `-_setEnabled:forFeature:`). Every
//! call checks the selector first, so a WebKit without them leaves the defaults in place.
//! App Store builds leave this module out (`platform/mod.rs`).

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{MainThreadMarker, msg_send, sel};
use objc2_foundation::NSString;
use objc2_web_kit::WKWebViewConfiguration;
use tauri::{Runtime, WebviewWindowBuilder};

/// The features the popover's webview is built with.
pub const DEFAULTS: &[(&str, bool)] = &[("AcceleratedDrawingEnabled", false)];

/// `builder` with a configuration carrying [`DEFAULTS`]. Call on the main thread; off it,
/// or in a bench launch measuring WebKit's own defaults, the builder is returned as is.
pub fn configure<'a, R: Runtime, M: tauri::Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    if !crate::bench::webkit_defaults() {
        return builder;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("webview configuration built off the main thread; WebKit defaults kept");
        return builder;
    };
    // SAFETY: `new` on the main thread (`mtm`) returns a fresh, unshared configuration.
    let config = unsafe { WKWebViewConfiguration::new(mtm) };
    // SAFETY: a fresh configuration always has preferences.
    let prefs: Retained<AnyObject> = unsafe { msg_send![&*config, preferences] };
    let wanted: Vec<(String, bool)> = DEFAULTS.iter().map(|&(k, v)| (k.to_owned(), v)).collect();
    // SAFETY: `prefs` is a live WKPreferences and this is the main thread.
    unsafe { set_on(&prefs, &wanted) };
    builder.with_webview_configuration(config)
}

/// Sets `features` on a live webview and reloads it, so layers that already exist are
/// rebuilt: the benchmark's A/B switch (`bench::webkit`).
#[cfg(feature = "bench")]
pub fn set_and_reload<R: Runtime>(webview: &tauri::Webview<R>, features: Vec<(String, bool)>) {
    let r = webview.with_webview(move |pw| {
        let wk: *mut AnyObject = pw.inner().cast();
        // SAFETY: wry hands its live WKWebView to this closure on the main thread.
        let Some(wk) = (unsafe { wk.as_ref() }) else {
            return;
        };
        // SAFETY: WKWebView's `configuration` and its `preferences` are public API and
        // never nil; `reload` is public API and its navigation is unused.
        unsafe {
            let config: Retained<AnyObject> = msg_send![wk, configuration];
            let prefs: Retained<AnyObject> = msg_send![&*config, preferences];
            set_on(&prefs, &features);
            let _: Option<Retained<AnyObject>> = msg_send![wk, reload];
        }
    });
    if let Err(e) = r {
        tracing::warn!("setting WebKit features: {e}");
    }
}

/// # Safety
///
/// `prefs` is a live `WKPreferences`, and this runs on the main thread.
unsafe fn set_on(prefs: &AnyObject, features: &[(String, bool)]) {
    let Some(class) = AnyClass::get(c"WKPreferences") else {
        return;
    };
    if class.class_method(sel!(_features)).is_none()
        || class
            .instance_method(sel!(_setEnabled:forFeature:))
            .is_none()
    {
        tracing::warn!("WebKit has no feature SPI; keeping its defaults");
        return;
    }
    // SAFETY: `_features` exists (checked) and returns an NSArray of _WKFeature.
    let all: Option<Retained<AnyObject>> = unsafe { msg_send![class, _features] };
    let Some(all) = all else {
        return;
    };
    // SAFETY: `all` is an NSArray; `count` and `objectAtIndex:` are NSArray's.
    let n: usize = unsafe { msg_send![&*all, count] };
    for (key, on) in features {
        let found = (0..n).find_map(|i| {
            // SAFETY: `i` is below `count`; a _WKFeature answers `key` with an NSString.
            let f: Retained<AnyObject> = unsafe { msg_send![&*all, objectAtIndex: i] };
            let k: Retained<NSString> = unsafe { msg_send![&*f, key] };
            (k.to_string() == *key).then_some(f)
        });
        match found {
            // SAFETY: `_setEnabled:forFeature:` exists (checked) and takes a BOOL and a
            // _WKFeature from `_features`.
            Some(f) => unsafe {
                let _: () = msg_send![prefs, _setEnabled: Bool::new(*on), forFeature: &*f];
            },
            None => tracing::warn!(%key, "unknown WebKit feature; skipped"),
        }
    }
}
