//! An in-process fake of the app's query socket, and a runner for the real
//! `statglass-cli` binary pointed at it through `STATGLASS_APP_DATA`.

#![allow(dead_code, reason = "each test binary uses a different subset")]

use std::{
    fs,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::{Path, PathBuf},
    process::{Command, Output},
    thread::JoinHandle,
};

use statglass_query::{
    API_VERSION, Answer, Body, CliErrorEnvelope, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, QueryError,
    QueryRequest, QueryResponse, frame,
};
use tempfile::TempDir;

pub enum Reply {
    Response(Box<QueryResponse>),
    /// A frame whose body is not JSON.
    Garbage,
}

pub fn ok(answer: Answer) -> Reply {
    Reply::Response(Box::new(QueryResponse {
        api: API_VERSION,
        app_version: "9.9.9".into(),
        body: Body::Ok(answer),
    }))
}

pub fn err(error: QueryError) -> Reply {
    Reply::Response(Box::new(QueryResponse {
        api: API_VERSION,
        app_version: "9.9.9".into(),
        body: Body::Error(error),
    }))
}

/// A temp app data directory with optional sockets bound in it.
pub struct Fake {
    dir: TempDir,
    server: Option<JoinHandle<QueryRequest>>,
    _control: Option<UnixListener>,
}

/// A private directory (0700) under the app data dir.
fn private_dir(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

fn bind_private(path: &Path) -> UnixListener {
    let listener = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    listener
}

impl Fake {
    /// The app data directory exists; nothing is bound (the app is not running).
    pub fn empty() -> Self {
        // Under /tmp: socket paths must stay under the 104-byte sun_path limit.
        let dir = tempfile::Builder::new()
            .prefix("sg")
            .tempdir_in("/tmp")
            .unwrap();
        Self {
            dir,
            server: None,
            _control: None,
        }
    }

    /// The query socket answers one request with `reply`.
    pub fn serving(reply: Reply) -> Self {
        let mut fake = Self::empty();
        let cli = private_dir(fake.dir.path(), "cli");
        let listener = bind_private(&cli.join("query.sock"));
        fake.server = Some(std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request: QueryRequest = frame::read_frame(&mut stream, MAX_REQUEST_BYTES).unwrap();
            match reply {
                Reply::Response(r) => {
                    frame::write_frame(&mut stream, &*r, MAX_RESPONSE_BYTES).unwrap()
                }
                Reply::Garbage => {
                    use std::io::Write;
                    stream.write_all(&7u32.to_be_bytes()).unwrap();
                    stream.write_all(b"garbage").unwrap();
                }
            }
            request
        }));
        fake
    }

    /// Only the app's control socket is bound: running, command-line access off.
    pub fn control_only() -> Self {
        let mut fake = Self::empty();
        let remote = private_dir(fake.dir.path(), "remote");
        fake._control = Some(bind_private(&remote.join("control.sock")));
        fake
    }

    pub fn app_data(&self) -> &Path {
        self.dir.path()
    }

    /// Runs the CLI with `args` against this app data directory.
    pub fn run(&self, args: &[&str]) -> Run {
        run_with(args, |c| {
            c.env("STATGLASS_APP_DATA", self.dir.path());
        })
    }

    /// The request the fake received. Call only when the CLI was expected to connect.
    pub fn request(self) -> QueryRequest {
        self.server.expect("not serving").join().unwrap()
    }
}

/// A finished CLI run.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The envelope on the last stderr line.
    pub fn envelope(&self) -> CliErrorEnvelope {
        let last = self.stderr.lines().last().expect("stderr is empty");
        serde_json::from_str(last).unwrap_or_else(|e| panic!("{e}: {last}"))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("{e}: {}", self.stdout))
    }
}

/// Runs the CLI with `args`, letting `setup` add environment or a working directory.
/// stdout is a pipe, so the audience is agent unless `--json` is passed.
pub fn run_with(args: &[&str], setup: impl FnOnce(&mut Command)) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_statglass-cli"));
    cmd.args(args).env_remove("STATGLASS_APP_ID");
    setup(&mut cmd);
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().unwrap();
    Run {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}
