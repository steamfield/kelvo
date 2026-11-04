//! The blocking client: one request per connection over `cli/query.sock`, std only.
//!
//! The client cannot check the peer's credentials without unsafe FFI, so it checks the
//! socket's directory instead: owned by this user and mode 0700, with the socket owned
//! by this user and mode 0600. Only this user can create a socket there, which is the
//! property a peer check would give.

use std::{
    io,
    os::unix::{
        fs::{FileTypeExt, MetadataExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    AppPaths, Body, ErrorTag, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, QueryRequest, QueryResponse,
    frame::{self, FrameError},
};

/// How long the CLI waits for an answer: above the app's 20 s deadline for requests
/// forwarded to a remote host, so the app's typed answer arrives first.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(25);

/// The longest bind error the client reports from the app's bind-failed marker.
pub const MAX_BIND_ERROR_CHARS: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("neither STATGLASS_APP_DATA nor HOME is set")]
    NoHome,
    #[error("no StatGlass data directory at {0}")]
    AppNotInstalled(PathBuf),
    #[error("nothing is answering at {0}")]
    AppNotRunning(PathBuf),
    #[error("StatGlass is running but {0} is not bound")]
    AccessDisabled(PathBuf),
    #[error("refusing to connect to {path}: {reason}")]
    UnsafeSocket { path: PathBuf, reason: &'static str },
    /// The app is running with access on but could not bind its socket; `error` is the
    /// first line of what it recorded, capped at [`MAX_BIND_ERROR_CHARS`].
    #[error("StatGlass could not bind {path}: {error}")]
    BindFailed { path: PathBuf, error: String },
    /// The app is running but has no query socket and never turned access off: it is
    /// older than the CLI.
    #[error("the running StatGlass predates the command-line tool ({0} is not bound)")]
    AppPredatesCli(PathBuf),
    #[error("no answer within the deadline")]
    Timeout,
    #[error("StatGlass {app_version} (api {api}) does not support this request")]
    AppOutdated { app_version: String, api: u16 },
    #[error("socket connection failed: {0}")]
    Connect(#[source] io::Error),
    #[error(transparent)]
    Frame(FrameError),
}

impl ClientError {
    /// The error tag, and so the exit code, this failure reports as.
    pub fn tag(&self) -> ErrorTag {
        match self {
            ClientError::AppNotInstalled(_) => ErrorTag::AppNotInstalled,
            ClientError::AppNotRunning(_) => ErrorTag::AppNotRunning,
            ClientError::AccessDisabled(_) => ErrorTag::AccessDisabled,
            ClientError::UnsafeSocket { .. } | ClientError::BindFailed { .. } => {
                ErrorTag::AccessUnavailable
            }
            // A socket we may not open is one the app cannot serve either; any other
            // connect failure means nothing is listening.
            ClientError::Connect(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                ErrorTag::AccessUnavailable
            }
            ClientError::Connect(_) => ErrorTag::AppNotRunning,
            ClientError::Timeout => ErrorTag::Busy,
            ClientError::AppOutdated { .. } | ClientError::AppPredatesCli(_) => {
                ErrorTag::AppOutdated
            }
            ClientError::NoHome | ClientError::Frame(_) => ErrorTag::Defect,
        }
    }
}

impl From<FrameError> for ClientError {
    fn from(e: FrameError) -> Self {
        if e.is_timeout() {
            ClientError::Timeout
        } else {
            ClientError::Frame(e)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    paths: AppPaths,
}

impl Client {
    /// A client for the app this executable belongs to (see [`AppPaths::discover`]).
    pub fn discover() -> Result<Self, ClientError> {
        AppPaths::discover()
            .map(Self::new)
            .ok_or(ClientError::NoHome)
    }

    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    /// Sends one request and reads its answer.
    ///
    /// Failure classification, in order: no app data directory is `AppNotInstalled`; a
    /// missing or refusing query socket is `AppNotRunning` when `control.sock` does not
    /// connect either, `AccessDisabled` when it does and the app's access-disabled marker
    /// is present, `AccessUnavailable` when the app's bind-failed marker is, and
    /// `AppOutdated` (an app older than the CLI) when it does without either; a socket
    /// that fails the ownership check is `AccessUnavailable`; a deadline passing is `Busy`.
    /// A connection the app closes without answering is classified like a missing socket
    /// when the socket is gone, and is `Busy` while it is still there.
    /// An `Unsupported` answer means the running app predates an operation this CLI
    /// sent, which is `AppOutdated`. Every other answer, error answers included, is
    /// returned as the response.
    pub fn request(
        &self,
        request: &QueryRequest,
        timeout: Duration,
    ) -> Result<QueryResponse, ClientError> {
        let socket = self.paths.query_socket();
        if !self.paths.app_data().is_dir() {
            return Err(ClientError::AppNotInstalled(
                self.paths.app_data().to_owned(),
            ));
        }
        if std::fs::symlink_metadata(&socket).is_err() {
            return Err(self.not_answering(socket));
        }
        check_private(&socket)?;
        let mut stream = match UnixStream::connect(&socket) {
            Ok(s) => s,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
                ) =>
            {
                return Err(self.not_answering(socket));
            }
            Err(e) => return Err(ClientError::Connect(e)),
        };
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|()| stream.set_write_timeout(Some(timeout)))
            .map_err(ClientError::Connect)?;
        // An app at its connection cap answers `Busy` and closes without reading, so a
        // request that could not be written may still have an answer waiting. A write
        // that ran out of time has no such answer: the app is not reading.
        let written = frame::write_frame(&mut stream, request, MAX_REQUEST_BYTES);
        if let Err(e) = &written
            && e.is_timeout()
        {
            return Err(ClientError::Timeout);
        }
        let response: QueryResponse = match frame::read_frame(&mut stream, MAX_RESPONSE_BYTES) {
            Ok(response) => response,
            Err(e) if e.is_timeout() => return Err(ClientError::Timeout),
            Err(e) if is_closed(&e) || written.as_ref().err().is_some_and(is_closed) => {
                return Err(self.closed_unanswered(socket));
            }
            Err(e) => return Err(written.err().unwrap_or(e).into()),
        };
        if let Body::Error(e) = &response.body
            && e.tag == ErrorTag::Unsupported
        {
            return Err(ClientError::AppOutdated {
                app_version: response.app_version,
                api: response.api,
            });
        }
        Ok(response)
    }

    /// The app closed the connection without answering: access was turned off or the app
    /// is quitting (the socket is gone, or the control socket with it), or the app's read
    /// deadline passed while it was still serving, which is `Timeout`.
    fn closed_unanswered(&self, socket: PathBuf) -> ClientError {
        let serving = std::fs::symlink_metadata(&socket).is_ok()
            && UnixStream::connect(self.paths.control_socket()).is_ok();
        if serving {
            return ClientError::Timeout;
        }
        match self.not_answering(socket) {
            // The socket was serving a moment ago, so this app has the CLI: with no
            // marker it is quitting, not older than the CLI.
            ClientError::AppPredatesCli(socket) => ClientError::AppNotRunning(socket),
            e => e,
        }
    }

    /// The query socket is absent or refusing. The app's always-bound control socket
    /// tells running from not running; while running, the marker the app writes when
    /// command-line access is off tells "access off", and the one it writes while binding
    /// fails tells "could not bind", from an app that predates the CLI.
    fn not_answering(&self, socket: PathBuf) -> ClientError {
        if UnixStream::connect(self.paths.control_socket()).is_err() {
            ClientError::AppNotRunning(socket)
        } else if self.paths.access_disabled_marker().exists() {
            ClientError::AccessDisabled(socket)
        } else if let Some(error) = bind_error(&self.paths.bind_failed_marker()) {
            ClientError::BindFailed {
                path: socket,
                error,
            }
        } else {
            ClientError::AppPredatesCli(socket)
        }
    }
}

/// Whether a frame failed because the peer closed or reset the connection.
fn is_closed(e: &FrameError) -> bool {
    match e {
        FrameError::Truncated => true,
        FrameError::Io(e) => matches!(
            e.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
        ),
        _ => false,
    }
}

/// The first line of the bind-failed marker at `path`, capped; `None` when there is none.
fn bind_error(path: &Path) -> Option<String> {
    let text = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&text);
    let line = text.lines().next().unwrap_or("").trim();
    let line: String = line.chars().take(MAX_BIND_ERROR_CHARS).collect();
    Some(if line.is_empty() {
        "no error recorded".to_owned()
    } else {
        line
    })
}

/// The socket's directory is a real directory owned by us with mode 0700, and the socket
/// is a socket owned by us with mode 0600.
fn check_private(socket: &Path) -> Result<(), ClientError> {
    let uid = rustix::process::geteuid().as_raw();
    let unsafe_socket = |path: &Path, reason| ClientError::UnsafeSocket {
        path: path.to_owned(),
        reason,
    };
    let dir = socket
        .parent()
        .ok_or_else(|| unsafe_socket(socket, "socket path has no directory"))?;
    let meta = std::fs::symlink_metadata(dir)
        .map_err(|_| unsafe_socket(dir, "directory is unreadable"))?;
    if !meta.is_dir() || meta.uid() != uid {
        return Err(unsafe_socket(dir, "directory is not owned by this user"));
    }
    if meta.mode() & 0o7777 != 0o700 {
        return Err(unsafe_socket(dir, "directory mode is not 0700"));
    }
    let meta = std::fs::symlink_metadata(socket)
        .map_err(|_| unsafe_socket(socket, "socket is unreadable"))?;
    if !meta.file_type().is_socket() || meta.uid() != uid {
        return Err(unsafe_socket(socket, "not a socket owned by this user"));
    }
    if meta.mode() & 0o7777 != 0o600 {
        return Err(unsafe_socket(socket, "socket mode is not 0600"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
