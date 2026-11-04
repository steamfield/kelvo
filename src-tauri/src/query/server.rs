//! The socket: bind, accept with a peer check, one framed request per connection, and the
//! concurrency and deadline rules every answer runs under.

use super::map::Fail;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use statglass_query::{
    API_VERSION, Body, ErrorDetails, ErrorTag, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, Op,
    QueryRequest, QueryResponse,
    frame::{FrameError, decode_len, encode_frame},
};
use statglass_remote::RemoteError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};

use super::map::{err, err_with, hint};
use super::{Backend, events, history, hosts, now, top};

/// Requests answered at once; one more gets `Busy`.
const MAX_IN_FLIGHT: usize = 4;
/// Connections open at once, most of them waiting for a slot or writing an answer. One
/// more is answered `Busy` and closed.
pub(super) const MAX_CONNECTIONS: usize = 32;
/// How long a client has to send its request.
const READ_REQUEST_WITHIN: Duration = Duration::from_secs(5);

/// Concurrency and deadlines. A slot is held until the handler's blocking work actually
/// ends, not released at the deadline: SQLite work on the blocking pool cannot be
/// cancelled, and releasing early would let retrying agents pile abandoned reads onto the
/// readers the dashboard uses.
pub(crate) struct Limits {
    slots: Arc<Semaphore>,
    /// Local and mirror reads.
    pub(super) local_deadline: Duration,
    /// Requests forwarded to a remote host: above the session's own 15 s, so its typed
    /// busy or unavailable answer arrives first.
    pub(super) remote_deadline: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(Duration::from_secs(10), Duration::from_secs(20))
    }
}

impl Limits {
    pub(super) fn new(local_deadline: Duration, remote_deadline: Duration) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            local_deadline,
            remote_deadline,
        }
    }

    /// A slot, or `None` when every one is taken.
    pub(super) fn try_slot(&self) -> Option<OwnedSemaphorePermit> {
        self.slots.clone().try_acquire_owned().ok()
    }

    #[cfg(test)]
    pub(super) fn free_slots(&self) -> usize {
        self.slots.available_permits()
    }
}

/// Runs `f` on the blocking pool with `slot` held until it returns. `Busy` past
/// `deadline`; the work keeps its slot until it ends.
pub(super) async fn run_blocking<T: Send + 'static>(
    slot: OwnedSemaphorePermit,
    deadline: Duration,
    f: impl FnOnce() -> Result<T, Fail> + Send + 'static,
) -> Result<T, Fail> {
    let task = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        f()
    });
    finish(tokio::time::timeout(deadline, task).await)
}

/// Runs `fut` as its own task with `slot` held until it ends; `Busy` past `deadline`.
pub(super) async fn run_async<T: Send + 'static>(
    slot: OwnedSemaphorePermit,
    deadline: Duration,
    fut: impl Future<Output = Result<T, Fail>> + Send + 'static,
) -> Result<T, Fail> {
    let task = tokio::spawn(async move {
        let _slot = slot;
        fut.await
    });
    finish(tokio::time::timeout(deadline, task).await)
}

fn finish<T>(
    joined: Result<Result<Result<T, Fail>, tokio::task::JoinError>, tokio::time::error::Elapsed>,
) -> Result<T, Fail> {
    match joined {
        Err(_) => Err(err(ErrorTag::Busy)),
        Ok(Err(e)) => Err(hint(ErrorTag::Defect, format!("query task failed: {e}"))),
        Ok(Ok(answer)) => answer,
    }
}

/// The answer to one decoded request.
pub(super) async fn answer(
    backend: Arc<dyn Backend>,
    limits: &Limits,
    request: QueryRequest,
) -> Body {
    if request.api > API_VERSION || matches!(request.op, Op::Unknown) {
        return Body::Error(*unsupported(backend.app_version(), None));
    }
    let Some(slot) = limits.try_slot() else {
        return Body::Error(*err(ErrorTag::Busy));
    };
    let result = match request.op {
        Op::Top { .. } => top::answer(backend, slot, limits, request.op).await,
        op => {
            run_blocking(slot, limits.local_deadline, move || {
                dispatch(backend.as_ref(), op)
            })
            .await
        }
    };
    match result {
        Ok(answer) => Body::Ok(answer),
        Err(e) => Body::Error(*e),
    }
}

/// `Unsupported`, with this app's api and version: a request from a newer CLI.
fn unsupported(app_version: String, hint: Option<String>) -> Fail {
    err_with(
        ErrorTag::Unsupported,
        ErrorDetails {
            api: Some(API_VERSION),
            app_version: Some(app_version),
            hint,
            ..Default::default()
        },
    )
}

/// The request in `body`. Its `api` is read first: a newer api, or a body this build
/// cannot decode (a newer op's fields, or a known op with a value added since), is
/// `Unsupported`, because the CLI validated its input before sending. Only a frame that
/// is not a JSON object at all is `Usage`.
pub(super) fn decode_request(
    body: &[u8],
    app_version: impl FnOnce() -> String,
) -> Result<QueryRequest, Fail> {
    #[derive(serde::Deserialize)]
    struct Head {
        api: u16,
    }
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| hint(ErrorTag::Usage, format!("unreadable request: {e}")))?;
    if !value.is_object() {
        return Err(hint(ErrorTag::Usage, "a request is a JSON object"));
    }
    let newer = |e: String| unsupported(app_version(), Some(e));
    match Head::deserialize(&value) {
        Ok(head) if head.api > API_VERSION => {
            return Err(newer(format!(
                "api {} is newer than {API_VERSION}",
                head.api
            )));
        }
        Ok(_) => {}
        Err(e) => return Err(newer(e.to_string())),
    }
    QueryRequest::deserialize(value).map_err(|e| newer(e.to_string()))
}

/// Every operation answered on the blocking pool.
fn dispatch(backend: &dyn Backend, op: Op) -> Result<statglass_query::Answer, Fail> {
    use statglass_query::Answer;
    match op {
        Op::Status => Ok(Answer::Status(hosts::status(backend))),
        Op::Hosts => Ok(Answer::Hosts(hosts::hosts(backend))),
        Op::Now { host, all, detail } => {
            now::now(backend, host.as_deref(), all, detail).map(Answer::Now)
        }
        Op::Metrics { host, module } => {
            hosts::metrics(backend, host.as_deref(), module.as_deref()).map(Answer::Metrics)
        }
        Op::History {
            host,
            series,
            from_ms,
            to_ms,
            points,
        } => history::answer(
            backend,
            &history::Ask {
                host: host.as_deref(),
                series: &series,
                from_ms,
                to_ms,
                points,
            },
        )
        .map(Answer::History),
        Op::Events {
            host,
            from_ms,
            to_ms,
            kind,
        } => events::answer(backend, host.as_deref(), from_ms, to_ms, kind.as_deref())
            .map(Answer::Events),
        Op::Top { .. } | Op::Unknown => Err(err(ErrorTag::Defect)),
    }
}

/// The response frame for `body`. An answer over the response cap becomes a `Usage`
/// error telling the agent to ask for less.
fn response_frame(app_version: String, body: Body) -> Result<Vec<u8>, FrameError> {
    let response = QueryResponse {
        api: API_VERSION,
        app_version,
        body,
    };
    match encode_frame(&response, MAX_RESPONSE_BYTES) {
        Err(FrameError::TooLarge { .. }) => {
            let body = Body::Error(*hint(ErrorTag::Usage, "narrow the range or series"));
            encode_frame(&QueryResponse { body, ..response }, MAX_RESPONSE_BYTES)
        }
        frame => frame,
    }
}

/// Reads one request, answers it and closes.
async fn connection(mut stream: UnixStream, backend: Arc<dyn Backend>, limits: Arc<Limits>) {
    let app_version = backend.app_version();
    let request = tokio::time::timeout(
        READ_REQUEST_WITHIN,
        read_request(&mut stream, move || app_version),
    )
    .await;
    let body = match request {
        Err(_) => return,
        Ok(Err(None)) => return,
        Ok(Err(Some(e))) => Body::Error(*e),
        Ok(Ok(request)) => answer(backend.clone(), &limits, request).await,
    };
    match response_frame(backend.app_version(), body) {
        Ok(frame) => {
            if let Err(e) = stream.write_all(&frame).await {
                tracing::debug!("CLI query client went away before its answer: {e}");
            }
            let _ = stream.shutdown().await;
        }
        Err(e) => tracing::warn!("encoding a CLI query answer: {e}"),
    }
}

/// The request, a typed error to answer with, or `None` when the client went away.
async fn read_request(
    stream: &mut UnixStream,
    app_version: impl FnOnce() -> String,
) -> Result<QueryRequest, Option<Fail>> {
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix).await.map_err(|_| None)?;
    let len = decode_len(prefix, MAX_REQUEST_BYTES)
        .map_err(|e| Some(hint(ErrorTag::Usage, e.to_string())))?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await.map_err(|_| None)?;
    decode_request(&body, app_version).map_err(Some)
}

/// Removes the socket file when serving stops.
struct Unlink(PathBuf);

impl Drop for Unlink {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Binds `path` owner-only.
pub(crate) fn bind(path: &Path) -> Result<UnixListener, RemoteError> {
    statglass_remote::bind_private(path)
}

/// Serves `listener`, bound at `path`, until `shutdown` changes; removes the socket file
/// when it stops.
pub(crate) async fn serve_on(
    listener: UnixListener,
    path: PathBuf,
    backend: Arc<dyn Backend>,
    limits: Arc<Limits>,
    mut shutdown: watch::Receiver<bool>,
) {
    let unlink = Unlink(path);
    // Pre-encoded: answering a connection over the cap must not cost the accept loop.
    let busy = response_frame(backend.app_version(), Body::Error(*err(ErrorTag::Busy)))
        .unwrap_or_default();
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            accepted = listener.accept() => {
                let stream = match accepted {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        tracing::warn!("CLI query socket accept failed; retrying: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                if statglass_remote::same_uid(&stream).is_err() {
                    continue;
                }
                if connections.len() >= MAX_CONNECTIONS {
                    refuse_busy(stream, &busy);
                    continue;
                }
                connections.spawn(connection(stream, backend.clone(), limits.clone()));
            },
            _ = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    // Unlink before closing connections, so a client whose connection closes finds the
    // socket gone and reports why rather than a busy app.
    drop(unlink);
    connections.abort_all();
    while connections.join_next().await.is_some() {}
}

/// Answers a connection over the cap with the pre-encoded `Busy` frame and closes it. The
/// write never waits: a peer whose buffer is full gets nothing, rather than holding the
/// accept loop. Written through the std socket, which stays non-blocking: tokio's
/// `try_write` reports a fresh stream not ready before the reactor has polled it.
fn refuse_busy(stream: UnixStream, busy: &[u8]) {
    use std::io::Write;
    let written = stream.into_std().and_then(|mut s| s.write(busy));
    if !matches!(written, Ok(n) if n == busy.len()) {
        tracing::debug!("CLI query client over the cap missed its Busy answer");
    }
}

/// Binds `path` owner-only and serves until `shutdown` changes.
#[cfg(test)]
pub(crate) async fn serve(
    path: PathBuf,
    backend: Arc<dyn Backend>,
    limits: Arc<Limits>,
    shutdown: watch::Receiver<bool>,
) -> Result<(), RemoteError> {
    let listener = bind(&path)?;
    serve_on(listener, path, backend, limits, shutdown).await;
    Ok(())
}
