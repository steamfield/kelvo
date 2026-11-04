//! The `statglass` CLI's query socket (D-126, `plan/rfc-agent-cli.md`): an owner-only Unix
//! socket at `<app_data>/cli/query.sock`, bound while "Allow command-line access" is on.
//! One length-prefixed JSON request per connection, answered from the same internals the
//! Tauri commands call. The contract is `statglass-query`; `map` turns internal types
//! into its DTOs in one place.
//!
//! | Need | Where |
//! |---|---|
//! | Bind, accept, frame, concurrency and deadlines | `server` |
//! | Host lookup, `status`, `hosts`, `metrics` | `hosts` |
//! | `now` and its headline set | `now` |
//! | `history` | `history` |
//! | Gap filtering, merging and `no_data` spans | `gaps` |
//! | `events` | `events` |
//! | `top` (local ring and durable usage, remote hosts live) | `top` |
//!
//! Handlers read through [`Backend`], which the app implements over [`AppState`] and the
//! tests over seeded stores. All SQLite work runs on the blocking pool; nothing here
//! touches the sampler.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use statglass_engine::LiveFrame;
use statglass_remote::{RemoteError, SyncWatermark};
use statglass_schema::durable_usage::{HistoricalUsageReply, HistoricalUsageRequest};
use statglass_schema::lock::LockExt;
use statglass_schema::usage::{UsageByApp, UsageKey};
use statglass_schema::{HostId, HostRecord};
use statglass_store::BucketRow;
use tauri::{AppHandle, Manager};

use crate::error::CommandError;
use crate::history::History;
use crate::remote::RemoteHostStatus;
use crate::state::AppState;

mod events;
mod gaps;
mod history;
mod hosts;
mod map;
mod now;
mod server;
mod top;

pub(crate) use server::{Limits, serve};

/// One host as the handlers see it, in `HostRegistry::all` order.
#[derive(Clone, Debug)]
pub(crate) struct HostView {
    pub record: HostRecord,
    /// `None` for this Mac.
    pub remote: Option<RemoteView>,
}

#[derive(Clone, Debug)]
pub(crate) struct RemoteView {
    /// The saved config's switch; a disabled host keeps its mirror but is not connected.
    pub enabled: bool,
    pub status: RemoteHostStatus,
}

/// A usage question forwarded to a remote host over its session (D-117).
#[derive(Clone, Debug)]
pub(crate) enum RemoteAsk {
    /// The host's last-hour usage ring.
    Ring {
        from_ms: i64,
        to_ms: i64,
        by: UsageKey,
        limit: u32,
    },
    /// The host's durable usage (D-120).
    Durable(HistoricalUsageRequest),
}

#[derive(Clone, Debug)]
pub(crate) enum RemoteAnswer {
    Ring(UsageByApp),
    Durable(HistoricalUsageReply),
}

pub(crate) type RemoteFuture =
    Pin<Box<dyn Future<Output = Result<RemoteAnswer, RemoteError>> + Send + 'static>>;

/// What the handlers read. Every method but [`Backend::remote_usage`] may block (SQLite)
/// and is only called on the blocking pool.
pub(crate) trait Backend: Send + Sync + 'static {
    fn local(&self) -> HostId;
    /// Every host: the registry's order (this Mac first, then by name), then saved hosts
    /// the registry does not hold.
    fn hosts(&self) -> Vec<HostView>;
    fn history(&self, host: HostId) -> Result<Arc<History>, CommandError>;
    /// The engine's (or the mirror's) rows not yet committed, for reads through now.
    fn recent_rows(&self, host: HostId, from_ms: i64, to_ms: i64) -> Vec<BucketRow>;
    /// Now on the host's clock.
    fn host_now(&self, host: HostId) -> i64;
    fn latest_frame(&self, host: HostId) -> Option<Arc<LiveFrame>>;
    fn sync_watermark(&self, host: HostId) -> Option<SyncWatermark>;
    /// This Mac's usage ring (`query_usage_by_app`'s source).
    fn usage_ring(
        &self,
        host: HostId,
        from_ms: i64,
        to_ms: i64,
        by: UsageKey,
        limit: usize,
    ) -> Result<UsageByApp, CommandError>;
    /// This Mac's durable usage (`query_historical_usage`'s source).
    fn durable_usage(
        &self,
        request: &HistoricalUsageRequest,
    ) -> Result<HistoricalUsageReply, CommandError>;
    /// Asks a remote host; async, never on the blocking pool.
    fn remote_usage(&self, host: HostId, ask: RemoteAsk) -> RemoteFuture;
    fn socket_path(&self) -> PathBuf;
    fn app_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}

/// [`Backend`] over the running app.
struct AppBackend {
    app: AppHandle,
    socket: PathBuf,
}

impl AppBackend {
    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }
}

impl Backend for AppBackend {
    fn local(&self) -> HostId {
        self.state().local
    }

    fn hosts(&self) -> Vec<HostView> {
        let state = self.state();
        let view = |record: HostRecord| {
            let remote = (!record.is_local).then(|| {
                let (enabled, status) = state
                    .remotes
                    .cli_view(record.id)
                    .unwrap_or_else(|| (true, RemoteHostStatus::default()));
                RemoteView { enabled, status }
            });
            HostView { record, remote }
        };
        let mut out: Vec<HostView> = state.hosts.all().iter().map(|h| view(h.record())).collect();
        let mut extra: Vec<HostRecord> = state
            .remotes
            .saved_records()
            .into_iter()
            .filter(|r| out.iter().all(|h| h.record.id != r.id))
            .collect();
        extra.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        out.extend(extra.into_iter().map(view));
        out
    }

    fn history(&self, host: HostId) -> Result<Arc<History>, CommandError> {
        self.state().history_for(host)
    }

    fn recent_rows(&self, host: HostId, from_ms: i64, to_ms: i64) -> Vec<BucketRow> {
        self.state().recent_rows(host, from_ms, to_ms)
    }

    fn host_now(&self, host: HostId) -> i64 {
        self.state().host_now(host)
    }

    fn latest_frame(&self, host: HostId) -> Option<Arc<LiveFrame>> {
        self.state().host(host).ok()?.latest_frame()
    }

    fn sync_watermark(&self, host: HostId) -> Option<SyncWatermark> {
        match self.state().remotes.sync_watermark(host) {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::debug!(%host, "reading the mirror watermark: {e}");
                None
            }
        }
    }

    fn usage_ring(
        &self,
        host: HostId,
        from_ms: i64,
        to_ms: i64,
        by: UsageKey,
        limit: usize,
    ) -> Result<UsageByApp, CommandError> {
        Ok(self
            .state()
            .local_host(host)?
            .usage_by_app(from_ms, to_ms, by, limit))
    }

    fn durable_usage(
        &self,
        request: &HistoricalUsageRequest,
    ) -> Result<HistoricalUsageReply, CommandError> {
        let state = self.state();
        let entry = state.local_host(request.host)?;
        Ok(state.history.store().historical_usage(request, || {
            entry.recent_usage_apps(request.from_ms, request.to_ms)
        })?)
    }

    fn remote_usage(&self, host: HostId, ask: RemoteAsk) -> RemoteFuture {
        let source = self.state().remotes.source(host);
        Box::pin(async move {
            let source =
                source.map_err(|e| RemoteError::Transport(format!("not connected: {e}")))?;
            match ask {
                RemoteAsk::Ring {
                    from_ms,
                    to_ms,
                    by,
                    limit,
                } => source
                    .usage_by_app(from_ms, to_ms, by, limit)
                    .await
                    .map(RemoteAnswer::Ring),
                RemoteAsk::Durable(request) => source
                    .historical_usage(request)
                    .await
                    .map(RemoteAnswer::Durable),
            }
        })
    }

    fn socket_path(&self) -> PathBuf {
        self.socket.clone()
    }
}

/// The socket's desired state, owned by one task that binds and unlinks it.
#[derive(Default)]
struct QuerySocket {
    desired: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
}

/// Binds the query socket when command-line access is on. Called once at startup, after
/// the app state is managed.
pub fn start(app: &AppHandle) {
    app.manage(QuerySocket::default());
    let enabled = app
        .state::<AppState>()
        .settings
        .settings()
        .cli_access
        .enabled;
    refresh(app, enabled);
}

/// Binds or unbinds the query socket. `enabled` comes from the caller, as with
/// [`crate::remote::refresh_metrics`]: the settings-change path cannot read it back.
pub fn refresh(app: &AppHandle, enabled: bool) {
    let Some(socket) = app.try_state::<QuerySocket>() else {
        return;
    };
    let mut desired = socket.desired.lock_ok();
    if let Some(tx) = desired.as_ref() {
        let _ = tx.send(enabled);
        return;
    }
    let paths = match app.path().app_data_dir() {
        Ok(dir) => statglass_query::AppPaths::new(dir),
        Err(e) => {
            tracing::warn!("no app data directory for the CLI query socket: {e}");
            return;
        }
    };
    let backend: Arc<dyn Backend> = Arc::new(AppBackend {
        app: app.clone(),
        socket: paths.query_socket(),
    });
    let (tx, rx) = tokio::sync::watch::channel(enabled);
    *desired = Some(tx);
    tauri::async_runtime::spawn(own_socket(paths, backend, rx));
}

/// Writes or removes `<app_data>/cli/access-disabled`, which tells the CLI that a
/// missing socket means access is off rather than an app older than the CLI.
fn mark_disabled(paths: &statglass_query::AppPaths, disabled: bool) {
    let marker = paths.access_disabled_marker();
    let result = if disabled {
        marker
            .parent()
            .map_or(Ok(()), statglass_store::create_private_dir)
            .and_then(|()| statglass_store::write_private(&marker, b""))
    } else {
        match std::fs::remove_file(&marker) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    };
    if let Err(e) = result {
        tracing::warn!("updating {}: {e}", marker.display());
    }
}

/// Serves while `desired` is true, rebinding after a failure; stops when it turns false
/// (leaving the access-disabled marker) and when its sender is dropped.
async fn own_socket(
    paths: statglass_query::AppPaths,
    backend: Arc<dyn Backend>,
    mut desired: tokio::sync::watch::Receiver<bool>,
) {
    let limits = Arc::new(Limits::default());
    let path = paths.query_socket();
    loop {
        if !*desired.borrow_and_update() {
            mark_disabled(&paths, true);
            if desired.changed().await.is_err() {
                break;
            }
            continue;
        }
        mark_disabled(&paths, false);
        let (stop, stopped) = tokio::sync::watch::channel(false);
        let server = serve(path.clone(), backend.clone(), limits.clone(), stopped);
        tokio::pin!(server);
        tokio::select! {
            result = &mut server => {
                if let Err(e) = result {
                    tracing::error!("CLI query socket: {e}");
                }
                tokio::select! {
                    () = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
                    changed = desired.changed() => if changed.is_err() { break },
                }
            }
            changed = desired.changed() => {
                let _ = stop.send(true);
                let _ = server.await;
                if changed.is_err() {
                    break;
                }
            }
        }
    }
}

/// Stops serving at exit; the server removes its socket file. Dropping the sender, rather
/// than sending false, leaves the access-disabled marker as the setting has it.
pub fn shutdown(app: &AppHandle) {
    if let Some(socket) = app.try_state::<QuerySocket>() {
        socket.desired.lock_ok().take();
    }
}

#[cfg(test)]
mod tests;
