//! Host lookup, and the `status`, `hosts` and `metrics` answers.

use super::map::Fail;
use statglass_query::{
    API_VERSION, ErrorDetails, ErrorTag, HostDto, HostsAnswer, MetricsAnswer, StatusAnswer,
    suggest::closest, time::timezone_name,
};
use statglass_schema::Module;

use super::map::{
    err_with, from_command, host_kind, host_ref, host_state, is_live, last_seen_ms, metric,
    metrics as catalog_metrics, rfc, sync_state, synced_through,
};
use super::{Backend, HostView};

/// The host `input` names: `None` or `local` for this Mac, else a display name, then a
/// host id, matched case-insensitively.
pub(super) fn resolve(backend: &dyn Backend, input: Option<&str>) -> Result<HostView, Fail> {
    let hosts = backend.hosts();
    let local = backend.local();
    let found = match input.map(str::trim) {
        None => hosts.iter().find(|h| h.record.id == local),
        Some(name) if name.eq_ignore_ascii_case("local") => {
            hosts.iter().find(|h| h.record.id == local)
        }
        Some(name) => hosts
            .iter()
            .find(|h| h.record.display_name.eq_ignore_ascii_case(name))
            .or_else(|| {
                hosts
                    .iter()
                    .find(|h| h.record.id.to_string().eq_ignore_ascii_case(name))
            }),
    };
    found.cloned().ok_or_else(|| {
        let input = input.unwrap_or("local");
        err_with(
            ErrorTag::UnknownHost,
            ErrorDetails {
                suggestions: closest(input, hosts.iter().map(|h| h.record.display_name.as_str())),
                host: Some(input.to_owned()),
                ..Default::default()
            },
        )
    })
}

/// `HostDisabled` for a remote host switched off in Settings.
pub(super) fn require_enabled(view: &HostView) -> Result<(), Fail> {
    if view.remote.as_ref().is_some_and(|r| !r.enabled) {
        return Err(err_with(
            ErrorTag::HostDisabled,
            ErrorDetails {
                host: Some(view.record.display_name.clone()),
                ..Default::default()
            },
        ));
    }
    Ok(())
}

/// [`resolve`], then [`require_enabled`].
pub(super) fn resolve_enabled(
    backend: &dyn Backend,
    input: Option<&str>,
) -> Result<HostView, Fail> {
    let view = resolve(backend, input)?;
    require_enabled(&view)?;
    Ok(view)
}

pub(super) fn status(backend: &dyn Backend) -> StatusAnswer {
    StatusAnswer {
        app_version: backend.app_version(),
        api: API_VERSION,
        timezone: timezone_name(),
        socket: backend.socket_path().display().to_string(),
        host_count: u32::try_from(backend.hosts().len()).unwrap_or(u32::MAX),
    }
}

pub(super) fn hosts(backend: &dyn Backend) -> HostsAnswer {
    let hosts = backend
        .hosts()
        .iter()
        .map(|view| host_dto(backend, view))
        .collect();
    HostsAnswer { hosts }
}

fn host_dto(backend: &dyn Backend, view: &HostView) -> HostDto {
    let r = &view.record;
    let watermark = view
        .remote
        .as_ref()
        .and_then(|_| backend.sync_watermark(r.id));
    // Informational: a host whose store cannot be read simply has no span.
    let span = backend
        .history(r.id)
        .ok()
        .and_then(|h| h.read(|reader| reader.retained_span(r.id)).ok())
        .flatten();
    // Stored rows lag the live frame by up to a rollup; while the host is live, its
    // history runs through the newest frame.
    let frame_ms = is_live(view)
        .then(|| backend.latest_frame(r.id))
        .flatten()
        .map(|f| f.ts_ms);
    let retained_to = span.map(|s| frame_ms.map_or(s.to_ms, |f| s.to_ms.max(f)));
    HostDto {
        id: r.id.to_string(),
        name: r.display_name.clone(),
        kind: host_kind(r),
        state: host_state(view),
        last_seen: last_seen_ms(view).map(rfc),
        sync: view.remote.as_ref().map(|v| sync_state(&v.status.sync)),
        synced_through: watermark.as_ref().and_then(synced_through).map(rfc),
        retained_from: span.map(|s| rfc(s.from_ms)),
        retained_to: retained_to.map(rfc),
        os_version: Some(r.info.os_version.clone()).filter(|v| !v.is_empty()),
        model: r.info.model.clone(),
        chip: r.info.chip.clone(),
    }
}

/// `metrics`: the catalog, optionally one module's, and with a host the series it has rows
/// for.
pub(super) fn metrics(
    backend: &dyn Backend,
    host: Option<&str>,
    module: Option<&str>,
) -> Result<MetricsAnswer, Fail> {
    let module = module.map(parse_module).transpose()?;
    let in_module =
        |id: &str| module.is_none_or(|m| metric(id).is_some_and(|d| d.module == m.as_str()));
    let metrics = catalog_metrics()
        .filter(|m| in_module(&m.id))
        .cloned()
        .collect();
    let Some(input) = host else {
        return Ok(MetricsAnswer {
            host: None,
            metrics,
            series: None,
        });
    };
    let view = resolve_enabled(backend, Some(input))?;
    let id = view.record.id;
    let history = backend.history(id).map_err(from_command)?;
    if !history.is_available() {
        return Err(super::map::err(ErrorTag::HistoryUnavailable));
    }
    let series = history
        .read(|r| r.stored_series(id))
        .map_err(from_command)?
        .iter()
        .filter(|k| in_module(k.metric.as_str()))
        .map(ToString::to_string)
        .collect();
    Ok(MetricsAnswer {
        host: Some(host_ref(&view.record)),
        metrics,
        series: Some(series),
    })
}

fn parse_module(input: &str) -> Result<Module, Fail> {
    Module::ALL
        .into_iter()
        .find(|m| m.as_str().eq_ignore_ascii_case(input.trim()))
        .ok_or_else(|| {
            err_with(
                ErrorTag::Usage,
                ErrorDetails {
                    suggestions: closest(input, Module::ALL.iter().map(|m| m.as_str())),
                    input: Some(input.to_owned()),
                    hint: Some(format!(
                        "modules: {}",
                        Module::ALL.map(Module::as_str).join(", ")
                    )),
                    ..Default::default()
                },
            )
        })
}
