use std::{
    fs::{self, Permissions},
    io::Write,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    thread,
};

use super::*;
use crate::{API_VERSION, Answer, HostsAnswer, Op, QueryError, frame::read_frame};

const SHORT: Duration = Duration::from_millis(300);

/// An app data dir in a temp dir, with a private `cli/` directory.
fn app_data() -> (tempfile::TempDir, Client) {
    let tmp = tempfile::tempdir().unwrap();
    let client = Client::new(AppPaths::new(tmp.path().join("app")));
    let cli = client.paths().query_socket().parent().unwrap().to_owned();
    fs::create_dir_all(&cli).unwrap();
    fs::set_permissions(&cli, Permissions::from_mode(0o700)).unwrap();
    (tmp, client)
}

fn bind(path: &Path) -> UnixListener {
    let l = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, Permissions::from_mode(0o600)).unwrap();
    l
}

/// Serves one connection: reads the request and answers with `answer(request)`.
fn serve_once(
    listener: UnixListener,
    answer: impl FnOnce(QueryRequest) -> Vec<u8> + Send + 'static,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let req: QueryRequest = read_frame(&mut stream, MAX_REQUEST_BYTES).unwrap();
        stream.write_all(&answer(req)).unwrap();
    })
}

fn response(body: Body) -> Vec<u8> {
    let r = QueryResponse {
        api: API_VERSION,
        app_version: "0.5.0".into(),
        body,
    };
    frame::encode_frame(&r, MAX_RESPONSE_BYTES).unwrap()
}

fn hosts() -> QueryRequest {
    QueryRequest::new(Op::Hosts)
}

#[test]
fn missing_app_data_is_not_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let client = Client::new(AppPaths::new(tmp.path().join("nope")));
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(matches!(e, ClientError::AppNotInstalled(_)), "{e:?}");
    assert_eq!(e.tag(), ErrorTag::AppNotInstalled);
}

#[test]
fn missing_socket_without_control_is_not_running_even_with_the_marker() {
    let (_tmp, client) = app_data();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert_eq!(e.tag(), ErrorTag::AppNotRunning, "{e:?}");
    // The app quit with access off: still not running.
    fs::write(client.paths().access_disabled_marker(), "").unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert_eq!(e.tag(), ErrorTag::AppNotRunning, "{e:?}");
}

#[test]
fn missing_socket_with_control_and_the_marker_is_access_disabled() {
    let (_tmp, client) = app_data();
    let control = client.paths().control_socket();
    fs::create_dir_all(control.parent().unwrap()).unwrap();
    let _control = bind(&control);
    fs::write(client.paths().access_disabled_marker(), "").unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(matches!(e, ClientError::AccessDisabled(_)), "{e:?}");
    assert_eq!(e.tag(), ErrorTag::AccessDisabled);
}

#[test]
fn missing_socket_with_control_and_no_marker_is_an_app_older_than_the_cli() {
    let (_tmp, client) = app_data();
    let control = client.paths().control_socket();
    fs::create_dir_all(control.parent().unwrap()).unwrap();
    let _control = bind(&control);
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(matches!(e, ClientError::AppPredatesCli(_)), "{e:?}");
    assert_eq!(e.tag(), ErrorTag::AppOutdated);
}

#[test]
fn missing_socket_with_control_and_a_bind_failure_is_access_unavailable() {
    let (_tmp, client) = app_data();
    let control = client.paths().control_socket();
    fs::create_dir_all(control.parent().unwrap()).unwrap();
    let _control = bind(&control);
    let long = "x".repeat(MAX_BIND_ERROR_CHARS + 50);
    fs::write(
        client.paths().bind_failed_marker(),
        format!("socket already serving {long}\nsecond line"),
    )
    .unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    let ClientError::BindFailed { error, .. } = &e else {
        panic!("{e:?}")
    };
    assert!(error.starts_with("socket already serving"), "{error}");
    assert_eq!(error.chars().count(), MAX_BIND_ERROR_CHARS);
    assert!(!error.contains("second line"));
    assert_eq!(e.tag(), ErrorTag::AccessUnavailable);
    assert_eq!(e.tag().exit_code(), 6);

    // Access turned off wins over a stale bind failure.
    fs::write(client.paths().access_disabled_marker(), "").unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert_eq!(e.tag(), ErrorTag::AccessDisabled, "{e:?}");
}

#[test]
fn connect_failures_are_not_running_unless_permission_is_refused() {
    let refused = ClientError::Connect(io::Error::from(io::ErrorKind::ConnectionReset));
    assert_eq!(refused.tag(), ErrorTag::AppNotRunning);
    let denied = ClientError::Connect(io::Error::from(io::ErrorKind::PermissionDenied));
    assert_eq!(denied.tag(), ErrorTag::AccessUnavailable);
}

#[test]
fn an_answer_after_a_failed_write_is_still_read() {
    let (_tmp, client) = app_data();
    let listener = bind(&client.paths().query_socket());
    let server = thread::spawn(move || {
        // Answer Busy and close without reading the request, as the app does at its
        // connection cap.
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .write_all(&response(Body::Error(QueryError::new(ErrorTag::Busy))))
            .unwrap();
    });
    // A request large enough not to fit in the socket buffer, so the write fails once the
    // server has closed.
    let big = QueryRequest::new(Op::History {
        host: None,
        series: vec!["x".repeat(MAX_REQUEST_BYTES / 2)],
        from_ms: 0,
        to_ms: 1,
        points: None,
    });
    let r = client.request(&big, SHORT).unwrap();
    server.join().unwrap();
    assert_eq!(r.body, Body::Error(QueryError::new(ErrorTag::Busy)));
}

#[test]
fn stale_socket_file_is_not_running() {
    let (_tmp, client) = app_data();
    drop(bind(&client.paths().query_socket()));
    assert!(client.paths().query_socket().exists());
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert_eq!(e.tag(), ErrorTag::AppNotRunning, "{e:?}");
}

#[test]
fn refuses_a_shared_directory_or_open_socket() {
    let (_tmp, client) = app_data();
    let socket = client.paths().query_socket();
    let _listener = bind(&socket);
    let dir = socket.parent().unwrap();

    fs::set_permissions(dir, Permissions::from_mode(0o755)).unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(
        matches!(
            e,
            ClientError::UnsafeSocket {
                reason: "directory mode is not 0700",
                ..
            }
        ),
        "{e:?}"
    );
    assert_eq!(e.tag(), ErrorTag::AccessUnavailable);

    fs::set_permissions(dir, Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&socket, Permissions::from_mode(0o666)).unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(
        matches!(
            e,
            ClientError::UnsafeSocket {
                reason: "socket mode is not 0600",
                ..
            }
        ),
        "{e:?}"
    );
}

#[test]
fn refuses_a_regular_file_in_place_of_the_socket() {
    let (_tmp, client) = app_data();
    let socket = client.paths().query_socket();
    fs::write(&socket, b"").unwrap();
    fs::set_permissions(&socket, Permissions::from_mode(0o600)).unwrap();
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(
        matches!(
            e,
            ClientError::UnsafeSocket {
                reason: "not a socket owned by this user",
                ..
            }
        ),
        "{e:?}"
    );
}

#[test]
fn round_trips_against_a_server() {
    let (_tmp, client) = app_data();
    let server = serve_once(bind(&client.paths().query_socket()), |req| {
        assert_eq!(req, hosts());
        response(Body::Ok(Answer::Hosts(HostsAnswer { hosts: vec![] })))
    });
    let r = client.request(&hosts(), SHORT).unwrap();
    server.join().unwrap();
    assert_eq!(r.app_version, "0.5.0");
    assert_eq!(
        r.body,
        Body::Ok(Answer::Hosts(HostsAnswer { hosts: vec![] }))
    );
}

#[test]
fn error_answers_are_returned_and_unsupported_means_app_outdated() {
    let (_tmp, client) = app_data();
    let busy = Body::Error(QueryError::new(ErrorTag::Busy));
    let server = serve_once(bind(&client.paths().query_socket()), {
        let busy = busy.clone();
        move |_| response(busy)
    });
    assert_eq!(client.request(&hosts(), SHORT).unwrap().body, busy);
    server.join().unwrap();

    fs::remove_file(client.paths().query_socket()).unwrap();
    let server = serve_once(bind(&client.paths().query_socket()), |_| {
        response(Body::Error(QueryError::new(ErrorTag::Unsupported)))
    });
    let e = client.request(&hosts(), SHORT).unwrap_err();
    server.join().unwrap();
    assert!(
        matches!(&e, ClientError::AppOutdated { app_version, api: 1 } if app_version == "0.5.0"),
        "{e:?}"
    );
    assert_eq!(e.tag(), ErrorTag::AppOutdated);
}

#[test]
fn silent_server_is_busy_and_garbage_is_a_defect() {
    let (_tmp, client) = app_data();
    let listener = bind(&client.paths().query_socket());
    let (timed_out, wait) = std::sync::mpsc::channel::<()>();
    let server = thread::spawn(move || {
        // Hold the first connection open without answering until the client gave up.
        let (stream, _) = listener.accept().unwrap();
        wait.recv().unwrap();
        drop(stream);
        let (mut stream, _) = listener.accept().unwrap();
        let _: QueryRequest = read_frame(&mut stream, MAX_REQUEST_BYTES).unwrap();
        stream.write_all(&[0, 0, 0, 2, b'{', b'{']).unwrap();
    });
    let e = client.request(&hosts(), SHORT).unwrap_err();
    timed_out.send(()).unwrap();
    assert!(matches!(e, ClientError::Timeout), "{e:?}");
    assert_eq!(e.tag(), ErrorTag::Busy);
    let e = client.request(&hosts(), SHORT).unwrap_err();
    assert!(
        matches!(e, ClientError::Frame(FrameError::Json(_))),
        "{e:?}"
    );
    assert_eq!(e.tag(), ErrorTag::Defect);
    server.join().unwrap();
}

#[test]
fn oversize_response_is_refused() {
    let (_tmp, client) = app_data();
    let server = serve_once(bind(&client.paths().query_socket()), |_| {
        ((MAX_RESPONSE_BYTES + 1) as u32).to_be_bytes().to_vec()
    });
    let e = client.request(&hosts(), SHORT).unwrap_err();
    server.join().unwrap();
    assert!(
        matches!(e, ClientError::Frame(FrameError::TooLarge { .. })),
        "{e:?}"
    );
}
