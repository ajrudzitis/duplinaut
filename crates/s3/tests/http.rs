//! Actual SDK requests against a local read-only HTTP fixture. Public dummy
//! credentials are injected explicitly; these tests never contact AWS.
use aws_sdk_s3::{
    config::{retry::RetryConfig, BehaviorVersion, Credentials, Region},
    Client,
};
use duplicity_core::source::{
    Cancellation, ObjectIdentity, ObjectSource, SourceError, SourceLimits, StoredObject,
};
use duplicity_s3::{S3Config, S3Source};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    bytes: Vec<u8>,
    length: usize,
    stall: Duration,
}
impl Response {
    fn body(bytes: impl Into<Vec<u8>>) -> Self {
        let bytes = bytes.into();
        Self {
            status: 200,
            length: bytes.len(),
            bytes,
            headers: vec![],
            stall: Duration::ZERO,
        }
    }
    fn object(bytes: impl Into<Vec<u8>>, etag: &str, version: Option<&str>) -> Self {
        let mut response = Self::body(bytes);
        response.headers.push(("ETag".into(), etag.into()));
        if let Some(version) = version {
            response
                .headers
                .push(("x-amz-version-id".into(), version.into()));
        }
        response
    }
    fn error(status: u16, code: &str) -> Self {
        let mut response = Self::body(
            format!("<Error><Code>{code}</Code><Message>fixture error</Message></Error>")
                .into_bytes(),
        );
        response.status = status;
        response
    }
}
struct Server {
    address: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    requests: mpsc::Receiver<String>,
}
impl Server {
    fn start(handler: impl Fn(&str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let handler = Arc::new(handler);
        let (sender, requests) = mpsc::channel();
        let thread = thread::spawn(move || {
            for connection in listener.incoming() {
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let mut stream = connection.unwrap();
                let handler = handler.clone();
                let sender = sender.clone();
                thread::spawn(move || {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut raw = Vec::new();
                    let mut byte = [0];
                    while !raw.ends_with(b"\r\n\r\n") && raw.len() < 65536 {
                        if stream.read(&mut byte).unwrap_or(0) == 0 {
                            return;
                        }
                        raw.push(byte[0]);
                    }
                    let request = String::from_utf8(raw).unwrap();
                    let _ = sender.send(request.clone());
                    let response = handler(&request);
                    let mut headers = format!(
                        "HTTP/1.1 {} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n",
                        response.status, response.length
                    );
                    for (name, value) in response.headers {
                        headers.push_str(&format!("{name}: {value}\r\n"));
                    }
                    headers.push_str("\r\n");
                    if stream.write_all(headers.as_bytes()).is_err() {
                        return;
                    }
                    thread::sleep(response.stall);
                    let _ = stream.write_all(&response.bytes);
                });
            }
        });
        Self {
            address,
            stop,
            thread: Some(thread),
            requests,
        }
    }
    fn source(&self) -> S3Source {
        self.source_with_timeout(Duration::from_secs(2))
    }
    fn source_with_timeout(&self, request_timeout: Duration) -> S3Source {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "PUBLICTESTKEY",
                "public-test-secret",
                None,
                None,
                "local-fixture",
            ))
            .endpoint_url(format!("http://{}", self.address))
            .force_path_style(true)
            .retry_config(RetryConfig::standard().with_max_attempts(1))
            .build();
        let mut settings = S3Config::new("fixture");
        settings.request_timeout = request_timeout;
        settings.stream_idle_timeout = Duration::from_millis(200);
        S3Source::from_client(settings, Client::from_conf(config)).unwrap()
    }
    fn request(&self) -> String {
        self.requests.recv_timeout(Duration::from_secs(2)).unwrap()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(&self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
fn object(size: u64) -> StoredObject {
    StoredObject {
        key: "backup/data".into(),
        size,
        identity: ObjectIdentity {
            etag: Some("\"etag\"".into()),
            version_id: None,
        },
    }
}
fn list_xml(key: &str, truncated: bool, token: Option<&str>) -> String {
    format!("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><IsTruncated>{truncated}</IsTruncated>{}<Contents><Key>{key}</Key><Size>3</Size><ETag>&quot;etag&quot;</ETag></Contents></ListBucketResult>", token.map(|token| format!("<NextContinuationToken>{token}</NextContinuationToken>")).unwrap_or_default())
}
#[test]
fn pagination_and_bounds_use_actual_sdk() {
    let server = Server::start(|request| {
        if request.contains("continuation-token=next") {
            Response::body(list_xml("backup/b", false, None))
        } else {
            Response::body(list_xml("backup/a", true, Some("next")))
        }
    });
    let source = server.source();
    let cancel = Cancellation::default();
    let items = source
        .list("backup/", SourceLimits::default(), &cancel)
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item.key.as_str())
            .collect::<Vec<_>>(),
        ["backup/a", "backup/b"]
    );
    let first = server.request();
    assert!(first.starts_with("GET /fixture") && first.contains("list-type=2"));
    assert!(server.request().contains("continuation-token=next"));
    let limits = SourceLimits {
        max_objects: 1,
        ..SourceLimits::default()
    };
    assert!(matches!(
        source.list("backup/", limits, &cancel),
        Err(SourceError::Limit)
    ));
    let limits = SourceLimits {
        max_metadata_bytes: 30,
        ..SourceLimits::default()
    };
    assert!(matches!(
        source.list("backup/", limits, &cancel),
        Err(SourceError::Limit)
    ));
}
#[test]
fn malformed_pagination_and_missing_identity_fail() {
    for xml in [list_xml("backup/a", true, None), list_xml("backup/a", true, Some("repeat")), list_xml("outside/a", false, None), "<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>backup/a</Key><Size>3</Size></Contents></ListBucketResult>".into()] {
        let server = Server::start(move |_| Response::body(xml.clone()));
        assert!(matches!(server.source().list("backup/", SourceLimits::default(), &Cancellation::default()), Err(SourceError::Backend(_))));
    }
}
// = annotation intentionally belongs to the test, not a generated parser.
//= docs/parsing-requirements.md#parse-source-001
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
#[test]
fn head_version_and_conditional_get_validate_identity() {
    let server =
        Server::start(|_| Response::object(b"abc".to_vec(), "\"etag\"", Some("version-42")));
    let source = server.source();
    let cancel = Cancellation::default();
    let stored = source
        .head("backup/data", SourceLimits::default(), &cancel)
        .unwrap();
    assert_eq!(stored.identity.version_id.as_deref(), Some("version-42"));
    assert!(server.request().starts_with("HEAD /fixture/backup/data "));
    let mut output = Vec::new();
    source
        .open(&stored, SourceLimits::default(), &cancel)
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, b"abc");
    let request = server.request().to_ascii_lowercase();
    let line = request.lines().next().unwrap();
    assert!(line.starts_with("get /fixture/backup/data?") && line.contains("versionid=version-42"));
    assert!(request.contains("if-match: \"etag\"\r\n"));
    assert!(request.contains("authorization:"));
}
#[test]
fn changed_missing_denied_and_unguarded_reads_fail() {
    for (status, code) in [
        (412, "PreconditionFailed"),
        (404, "NoSuchKey"),
        (403, "AccessDenied"),
    ] {
        let server = Server::start(move |_| Response::error(status, code));
        let result = server.source().open(
            &object(3),
            SourceLimits::default(),
            &Cancellation::default(),
        );
        match status {
            412 => assert!(matches!(result, Err(SourceError::Changed))),
            404 => assert!(matches!(result, Err(SourceError::Missing))),
            _ => assert!(matches!(result, Err(SourceError::Backend(_)))),
        }
    }
    for (index, response) in [
        Response::object(b"abc".to_vec(), "\"changed\"", None),
        Response::object(b"abcd".to_vec(), "\"etag\"", None),
        Response::object(b"abc".to_vec(), "\"etag\"", Some("other-version")),
    ]
    .into_iter()
    .enumerate()
    {
        let response = Arc::new(response);
        let server = Server::start(move |_| Response {
            status: response.status,
            headers: response.headers.clone(),
            bytes: response.bytes.clone(),
            length: response.length,
            stall: response.stall,
        });
        let mut stored = object(3);
        if index == 2 {
            stored.identity.version_id = Some("original-version".into());
        }
        assert!(matches!(
            server
                .source()
                .open(&stored, SourceLimits::default(), &Cancellation::default()),
            Err(SourceError::Changed)
        ));
    }
    let server = Server::start(|_| Response::object(b"abc".to_vec(), "\"etag\"", None));
    let mut stored = object(3);
    stored.identity.etag = None;
    assert!(matches!(
        server
            .source()
            .open(&stored, SourceLimits::default(), &Cancellation::default()),
        Err(SourceError::Backend(_))
    ));
    assert!(server
        .requests
        .recv_timeout(Duration::from_millis(100))
        .is_err());
}
#[test]
fn truncation_idle_timeout_and_network_cancellation_fail_promptly() {
    let server = Server::start(|_| {
        let mut response = Response::object(b"ab".to_vec(), "\"etag\"", None);
        response.length = 3;
        response
    });
    let mut bytes = Vec::new();
    assert!(server
        .source()
        .open(
            &object(3),
            SourceLimits::default(),
            &Cancellation::default()
        )
        .unwrap()
        .read_to_end(&mut bytes)
        .is_err());
    let server = Server::start(|_| {
        let mut response = Response::object(b"abc".to_vec(), "\"etag\"", None);
        response.stall = Duration::from_secs(1);
        response
    });
    let source = server.source();
    let cancel = Cancellation::default();
    let mut reader = source
        .open(&object(3), SourceLimits::default(), &cancel)
        .unwrap();
    let start = Instant::now();
    assert!(reader.read_to_end(&mut Vec::new()).is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
    let cancel = Cancellation::default();
    let mut reader = source
        .open(&object(3), SourceLimits::default(), &cancel)
        .unwrap();
    let cancelling = cancel.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        cancelling.cancel();
    });
    let start = Instant::now();
    assert!(reader.read(&mut [0; 3]).is_err());
    assert!(start.elapsed() < Duration::from_millis(500));
}
#[test]
fn reader_drop_does_not_cancel_other_operations_or_nested_runtime() {
    let server = Server::start(|_| Response::object(vec![7; 1024 * 1024], "\"etag\"", None));
    let source = server.source();
    let cancel = Cancellation::default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let reader = source
            .open(&object(1024 * 1024), SourceLimits::default(), &cancel)
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        drop(reader);
        assert!(!cancel.is_cancelled());
        let mut bytes = Vec::new();
        source
            .open(&object(1024 * 1024), SourceLimits::default(), &cancel)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, vec![7; 1024 * 1024]);
    });
}

#[test]
fn cancellation_interrupts_pending_headers_and_listing() {
    let server = Server::start(|request| {
        thread::sleep(Duration::from_secs(1));
        if request.contains("list-type=2") {
            Response::body(list_xml("backup/a", false, None))
        } else {
            Response::object(b"abc".to_vec(), "\"etag\"", None)
        }
    });
    let source = server.source();
    for listing in [false, true] {
        let cancel = Cancellation::default();
        let cancelling = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            cancelling.cancel();
        });
        let start = Instant::now();
        if listing {
            assert!(matches!(
                source.list("backup/", SourceLimits::default(), &cancel),
                Err(SourceError::Cancelled)
            ));
        } else {
            assert!(matches!(
                source.open(&object(3), SourceLimits::default(), &cancel),
                Err(SourceError::Cancelled)
            ));
        }
        assert!(start.elapsed() < Duration::from_millis(500));
    }
}

#[test]
fn request_timeout_and_head_metadata_bounds_are_enforced() {
    let server = Server::start(|_| {
        thread::sleep(Duration::from_secs(1));
        Response::object(b"abc".to_vec(), "\"etag\"", None)
    });
    let source = server.source_with_timeout(Duration::from_millis(100));
    let start = Instant::now();
    assert!(matches!(
        source.head(
            "backup/data",
            SourceLimits::default(),
            &Cancellation::default()
        ),
        Err(SourceError::Backend(_))
    ));
    assert!(start.elapsed() < Duration::from_millis(500));
    let server = Server::start(|_| {
        Response::object(
            b"abc".to_vec(),
            &format!("\"{}\"", "e".repeat(4096)),
            Some("v"),
        )
    });
    let limits = SourceLimits {
        max_metadata_bytes: 1024,
        ..SourceLimits::default()
    };
    assert!(matches!(
        server
            .source()
            .head("backup/data", limits, &Cancellation::default()),
        Err(SourceError::Limit)
    ));
}

#[test]
fn body_checksum_error_after_exact_bytes_remains_failure_on_retry() {
    let server = Server::start(|_| {
        let mut response = Response::object(b"abc".to_vec(), "\"etag\"", None);
        // A correctly encoded four-byte CRC32 value that disagrees with "abc".
        response
            .headers
            .push(("x-amz-checksum-crc32".into(), "AAAAAA==".into()));
        response
    });
    let mut reader = server
        .source()
        .open(
            &object(3),
            SourceLimits::default(),
            &Cancellation::default(),
        )
        .unwrap();
    let mut bytes = [0; 3];
    reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"abc");
    let initial = reader.read(&mut [0; 1]).unwrap_err();
    for _ in 0..3 {
        let repeated = reader.read(&mut [0; 1]).unwrap_err();
        assert_eq!(repeated.kind(), initial.kind());
        assert_eq!(repeated.to_string(), initial.to_string());
    }
}
