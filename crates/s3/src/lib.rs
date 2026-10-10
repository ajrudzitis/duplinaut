//! Read-only S3 storage. SDK, HTTP, credentials and scheduling are trusted I/O
//! boundaries; deterministic parser proofs do not prove these dependencies.
use aws_sdk_s3::{config::Region, error::ProvideErrorMetadata, Client};
use duplicity_core::source::{
    Cancellation, CheckedReader, ObjectIdentity, ObjectSource, SourceError, SourceLimits,
    StoredObject,
};
use std::{
    collections::HashSet,
    future::Future,
    io::{self, Read},
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::{io::AsyncReadExt, runtime::Handle};

const POLL: Duration = Duration::from_millis(20);
const CHUNK: usize = 64 * 1024;

/// Defaults use the SDK's usual credential/region chains. Custom endpoints and
/// path addressing support S3-compatible servers; no bucket writes are exposed.
#[derive(Clone, Debug)]
pub struct S3Config {
    pub bucket: String,
    pub prefix: String,
    pub region: Option<String>,
    pub profile: Option<String>,
    pub endpoint: Option<String>,
    pub force_path_style: bool,
    /// Covers credentials, retries and response headers for each API request.
    pub request_timeout: Duration,
    /// Maximum idle time awaiting the next body bytes (not queue backpressure).
    pub stream_idle_timeout: Duration,
}
impl S3Config {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: String::new(),
            region: None,
            profile: None,
            endpoint: None,
            force_path_style: false,
            request_timeout: Duration::from_secs(60),
            stream_idle_timeout: Duration::from_secs(30),
        }
    }
    fn validate(&self) -> Result<(), SourceError> {
        if self.bucket.is_empty()
            || self.request_timeout.is_zero()
            || self.stream_idle_timeout.is_zero()
        {
            return Err(SourceError::Backend("invalid S3 configuration".into()));
        }
        Ok(())
    }
}

/// Own the runtime on an ordinary thread. Its creation and destruction are
/// safe when the synchronous source is called from within another runtime.
struct Worker {
    handle: Handle,
    _keep_alive: mpsc::Sender<()>,
}
impl Worker {
    fn start() -> Result<Arc<Self>, SourceError> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (keep_alive, stopped) = mpsc::channel();
        std::thread::Builder::new()
            .name("duplicity-s3-runtime".into())
            .spawn(move || {
                match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => {
                        if ready_tx.send(Ok(runtime.handle().clone())).is_ok() {
                            let _ = stopped.recv();
                        }
                        runtime.shutdown_timeout(Duration::from_secs(1));
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
            })?;
        let handle = ready_rx
            .recv()
            .map_err(|_| SourceError::Backend("S3 runtime startup failed".into()))??;
        Ok(Arc::new(Self {
            handle,
            _keep_alive: keep_alive,
        }))
    }
    fn run<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, SourceError>> + Send + 'static,
        cancel: &Cancellation,
    ) -> Result<T, SourceError> {
        if cancel.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        let (sender, receiver) = mpsc::channel();
        let task = self.handle.spawn(async move {
            let _ = sender.send(future.await);
        });
        loop {
            if cancel.is_cancelled() {
                task.abort();
                return Err(SourceError::Cancelled);
            }
            match receiver.recv_timeout(POLL) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(SourceError::Backend("S3 worker terminated".into()))
                }
            }
        }
    }
}
async fn cancelled(cancel: &Cancellation, local: &Cancellation) {
    while !cancel.is_cancelled() && !local.is_cancelled() {
        tokio::time::sleep(POLL).await;
    }
}
async fn request<T>(
    future: impl Future<Output = Result<T, SourceError>>,
    timeout: Duration,
    cancel: &Cancellation,
    local: &Cancellation,
) -> Result<T, SourceError> {
    tokio::select! {
        _ = cancelled(cancel, local) => Err(SourceError::Cancelled),
        result = tokio::time::timeout(timeout, future) => result.map_err(|_| SourceError::Backend("S3 request timed out".into()))?,
    }
}
fn service_error(code: Option<&str>) -> SourceError {
    match code {
        Some("NoSuchKey" | "NoSuchVersion" | "NotFound" | "404") => SourceError::Missing,
        Some("PreconditionFailed" | "412") => SourceError::Changed,
        Some(code) => SourceError::Backend(format!("S3 service error: {code}")),
        None => SourceError::Backend("S3 request failed".into()),
    }
}

#[derive(Clone)]
pub struct S3Source {
    worker: Arc<Worker>,
    client: Client,
    config: S3Config,
}
impl S3Source {
    pub fn connect(config: S3Config, cancel: &Cancellation) -> Result<Self, SourceError> {
        config.validate()?;
        let worker = Worker::start()?;
        let settings = config.clone();
        let cancellation = cancel.clone();
        let client = worker.run(
            async move {
                request(
                    async {
                        let mut loader =
                            aws_config::defaults(aws_config::BehaviorVersion::latest());
                        if let Some(region) = settings.region {
                            loader = loader.region(Region::new(region));
                        }
                        if let Some(profile) = settings.profile {
                            loader = loader.profile_name(profile);
                        }
                        let shared = loader.load().await;
                        let mut builder = aws_sdk_s3::config::Builder::from(&shared)
                            .force_path_style(settings.force_path_style);
                        if let Some(endpoint) = settings.endpoint {
                            builder = builder.endpoint_url(endpoint);
                        }
                        Ok(Client::from_conf(builder.build()))
                    },
                    settings.request_timeout,
                    &cancellation,
                    &Cancellation::default(),
                )
                .await
            },
            cancel,
        )?;
        Ok(Self {
            worker,
            client,
            config,
        })
    }
    /// Inject a configured SDK client, for example static public test credentials.
    /// SDK client construction remains a trusted caller boundary.
    pub fn from_client(config: S3Config, client: Client) -> Result<Self, SourceError> {
        config.validate()?;
        Ok(Self {
            worker: Worker::start()?,
            client,
            config,
        })
    }
    fn check_key(&self, key: &str) -> Result<(), SourceError> {
        if !key.starts_with(&self.config.prefix) {
            return Err(SourceError::Backend(
                "object is outside source prefix".into(),
            ));
        }
        Ok(())
    }
    /// Discover a current immutable version where bucket versioning supplies it.
    pub fn head(
        &self,
        key: &str,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<StoredObject, SourceError> {
        self.check_key(key)?;
        if key.len() > limits.max_metadata_bytes {
            return Err(SourceError::Limit);
        }
        let source = self.clone();
        let key = key.to_owned();
        let cancellation = cancel.clone();
        self.worker.run(
            async move {
                let output = request(
                    async {
                        source
                            .client
                            .head_object()
                            .bucket(&source.config.bucket)
                            .key(&key)
                            .send()
                            .await
                            .map_err(|error| {
                                service_error(error.as_service_error().and_then(|e| e.code()))
                            })
                    },
                    source.config.request_timeout,
                    &cancellation,
                    &Cancellation::default(),
                )
                .await?;
                let etag = output
                    .e_tag()
                    .filter(|etag| !etag.is_empty())
                    .ok_or_else(|| {
                        SourceError::Backend("object has no conditional-read ETag".into())
                    })?;
                if metadata_cost(
                    key.len(),
                    etag.len(),
                    output.version_id().map_or(0, str::len),
                )? > limits.max_metadata_bytes
                {
                    return Err(SourceError::Limit);
                }
                Ok(StoredObject {
                    key,
                    size: positive_size(output.content_length())?,
                    identity: ObjectIdentity {
                        version_id: output.version_id().map(str::to_owned),
                        etag: Some(etag.to_owned()),
                    },
                })
            },
            cancel,
        )
    }
}
fn positive_size(size: Option<i64>) -> Result<u64, SourceError> {
    size.and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| SourceError::Backend("missing or negative object size".into()))
}
fn required_etag(value: Option<&str>) -> Result<String, SourceError> {
    value
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| SourceError::Backend("object has no conditional-read ETag".into()))
}
fn retained_bytes(object: &StoredObject) -> Result<usize, SourceError> {
    metadata_cost(
        object.key.len(),
        object.identity.etag.as_ref().map_or(0, String::len),
        object.identity.version_id.as_ref().map_or(0, String::len),
    )
}
fn metadata_cost(key: usize, etag: usize, version: usize) -> Result<usize, SourceError> {
    // Two struct slots cover Vec growth, plus conservative allocation overhead.
    key.checked_add(etag)
        .and_then(|n| n.checked_add(version))
        .and_then(|n| n.checked_add(2 * std::mem::size_of::<StoredObject>() + 128))
        .ok_or(SourceError::Limit)
}
impl ObjectSource for S3Source {
    fn list(
        &self,
        prefix: &str,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError> {
        if self
            .config
            .prefix
            .len()
            .checked_add(prefix.len())
            .ok_or(SourceError::Limit)?
            > limits.max_metadata_bytes
        {
            return Err(SourceError::Limit);
        }
        let prefix = format!("{}{prefix}", self.config.prefix);
        let source = self.clone();
        let cancellation = cancel.clone();
        self.worker.run(
            async move {
                let mut objects = Vec::new();
                let mut metadata = prefix
                    .len()
                    .checked_add(std::mem::size_of::<Vec<StoredObject>>())
                    .ok_or(SourceError::Limit)?;
                if metadata > limits.max_metadata_bytes {
                    return Err(SourceError::Limit);
                }
                let mut continuation = None;
                let mut tokens = HashSet::new();
                loop {
                    let output = request(
                        async {
                            source
                                .client
                                .list_objects_v2()
                                .bucket(&source.config.bucket)
                                .prefix(&prefix)
                                .max_keys(1000)
                                .set_continuation_token(continuation.clone())
                                .send()
                                .await
                                .map_err(|error| {
                                    service_error(error.as_service_error().and_then(|e| e.code()))
                                })
                        },
                        source.config.request_timeout,
                        &cancellation,
                        &Cancellation::default(),
                    )
                    .await?;
                    for item in output.contents() {
                        if cancellation.is_cancelled() {
                            return Err(SourceError::Cancelled);
                        }
                        if objects.len() >= limits.max_objects {
                            return Err(SourceError::Limit);
                        }
                        let key = item.key().ok_or_else(|| {
                            SourceError::Backend("listing has no object key".into())
                        })?;
                        if !key.starts_with(&prefix) {
                            return Err(SourceError::Backend(
                                "listing returned an out-of-prefix object".into(),
                            ));
                        }
                        let etag =
                            item.e_tag()
                                .filter(|etag| !etag.is_empty())
                                .ok_or_else(|| {
                                    SourceError::Backend(
                                        "object has no conditional-read ETag".into(),
                                    )
                                })?;
                        metadata = metadata
                            .checked_add(metadata_cost(key.len(), etag.len(), 0)?)
                            .ok_or(SourceError::Limit)?;
                        if metadata > limits.max_metadata_bytes {
                            return Err(SourceError::Limit);
                        }
                        let object = StoredObject {
                            key: key.to_owned(),
                            size: positive_size(item.size())?,
                            identity: ObjectIdentity {
                                etag: Some(etag.to_owned()),
                                version_id: None,
                            },
                        };
                        objects.push(object);
                    }
                    if !output.is_truncated().unwrap_or(false) {
                        return Ok(objects);
                    }
                    let token = output
                        .next_continuation_token()
                        .filter(|token| !token.is_empty())
                        .ok_or_else(|| {
                            SourceError::Backend(
                                "truncated listing has no continuation token".into(),
                            )
                        })?;
                    // Retain token history to reject cycles; count strings and hash slots.
                    metadata = metadata
                        .checked_add(token.len().saturating_mul(2).saturating_add(128))
                        .ok_or(SourceError::Limit)?;
                    if metadata > limits.max_metadata_bytes {
                        return Err(SourceError::Limit);
                    }
                    if !tokens.insert(token.to_owned()) {
                        return Err(SourceError::Backend("listing continuation cycle".into()));
                    }
                    continuation = Some(token.to_owned());
                }
            },
            cancel,
        )
    }
    //= docs/parsing-requirements.md#parse-source-001
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    fn open(
        &self,
        object: &StoredObject,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError> {
        self.check_key(&object.key)?;
        if object.size > limits.max_stored_bytes
            || retained_bytes(object)? > limits.max_metadata_bytes
        {
            return Err(SourceError::Limit);
        }
        let etag = required_etag(object.identity.etag.as_deref())?;
        let (headers_tx, headers_rx) = mpsc::channel();
        let (chunks_tx, chunks_rx) = mpsc::sync_channel(2);
        let source = self.clone();
        let object = object.clone();
        let expected = object.size;
        let cancellation = cancel.clone();
        let local = Cancellation::default();
        let producer_stop = local.clone();
        let task = self.worker.handle.spawn(async move {
            let response = request(
                async {
                    source
                        .client
                        .get_object()
                        .bucket(&source.config.bucket)
                        .key(&object.key)
                        .if_match(&etag)
                        .set_version_id(object.identity.version_id.clone())
                        .send()
                        .await
                        .map_err(|error| {
                            service_error(error.as_service_error().and_then(|e| e.code()))
                        })
                },
                source.config.request_timeout,
                &cancellation,
                &producer_stop,
            )
            .await;
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    let _ = headers_tx.send(Err(error));
                    return;
                }
            };
            let identity_matches = response.e_tag() == Some(etag.as_str())
                && object
                    .identity
                    .version_id
                    .as_ref()
                    .is_none_or(|version| response.version_id() == Some(version.as_str()));
            if !identity_matches
                || positive_size(response.content_length()).ok() != Some(object.size)
            {
                let _ = headers_tx.send(Err(SourceError::Changed));
                return;
            }
            if headers_tx.send(Ok(())).is_err() {
                return;
            }
            let mut body = response.body.into_async_read();
            loop {
                let mut bytes = vec![0u8; CHUNK];
                let read = request(
                    async { body.read(&mut bytes).await.map_err(SourceError::Io) },
                    source.config.stream_idle_timeout,
                    &cancellation,
                    &producer_stop,
                )
                .await;
                let message = match read {
                    Ok(0) => StreamMessage::End,
                    Ok(count) => {
                        bytes.truncate(count);
                        StreamMessage::Bytes(bytes)
                    }
                    Err(error) => StreamMessage::Error(io::Error::other(error)),
                };
                let terminal = !matches!(message, StreamMessage::Bytes(_));
                if !send_chunk(&chunks_tx, message, &cancellation, &producer_stop).await || terminal
                {
                    return;
                }
            }
        });
        loop {
            if cancel.is_cancelled() {
                local.cancel();
                task.abort();
                return Err(SourceError::Cancelled);
            }
            match headers_rx.recv_timeout(POLL) {
                Ok(result) => {
                    result?;
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(SourceError::Backend("S3 stream startup failed".into()))
                }
            }
        }
        let stream = StreamReader {
            receiver: chunks_rx,
            current: io::Cursor::new(Vec::new()),
            ended: false,
            failure: None,
            cancel: cancel.clone(),
            local,
            _worker: self.worker.clone(),
        };
        Ok(Box::new(CheckedReader::new(
            stream,
            expected,
            limits.max_stored_bytes,
            cancel.clone(),
        )?))
    }
}
enum StreamMessage {
    Bytes(Vec<u8>),
    Error(io::Error),
    End,
}
async fn send_chunk(
    sender: &mpsc::SyncSender<StreamMessage>,
    mut message: StreamMessage,
    cancel: &Cancellation,
    local: &Cancellation,
) -> bool {
    loop {
        if cancel.is_cancelled() || local.is_cancelled() {
            return false;
        }
        match sender.try_send(message) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
            Err(mpsc::TrySendError::Full(value)) => message = value,
        }
        tokio::time::sleep(POLL).await;
    }
}
struct StreamReader {
    receiver: mpsc::Receiver<StreamMessage>,
    current: io::Cursor<Vec<u8>>,
    ended: bool,
    failure: Option<(io::ErrorKind, String)>,
    cancel: Cancellation,
    local: Cancellation,
    _worker: Arc<Worker>,
}
impl StreamReader {
    fn fail(&mut self, error: io::Error) -> io::Error {
        self.failure = Some((error.kind(), error.to_string()));
        self.local.cancel();
        error
    }
}
impl Read for StreamReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.failure {
            return Err(io::Error::new(*kind, message.clone()));
        }
        loop {
            if self.cancel.is_cancelled() {
                return Err(io::Error::other("S3 stream cancelled"));
            }
            let count = Read::read(&mut self.current, buffer)?;
            if count > 0 {
                return Ok(count);
            }
            if self.ended {
                return Ok(0);
            }
            match self.receiver.recv_timeout(POLL) {
                Ok(StreamMessage::Bytes(bytes)) => self.current = io::Cursor::new(bytes),
                Ok(StreamMessage::End) => self.ended = true,
                Ok(StreamMessage::Error(error)) => {
                    return Err(self.fail(error));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.fail(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "S3 stream terminated without EOF",
                    )));
                }
            }
        }
    }
}
impl Drop for StreamReader {
    fn drop(&mut self) {
        self.local.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_queue_backpressure_is_interruptible_on_reader_drop() {
        let worker = Worker::start().unwrap();
        let (sender, receiver) = mpsc::sync_channel(2);
        let (done_tx, done_rx) = mpsc::channel();
        let cancel = Cancellation::default();
        let local = Cancellation::default();
        let producer_cancel = cancel.clone();
        let producer_local = local.clone();
        worker.handle.spawn(async move {
            for _ in 0..3 {
                if !send_chunk(
                    &sender,
                    StreamMessage::Bytes(vec![0; CHUNK]),
                    &producer_cancel,
                    &producer_local,
                )
                .await
                {
                    done_tx.send(()).unwrap();
                    return;
                }
            }
            panic!("third chunk must block behind the two-slot queue");
        });
        assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_err());
        let stream = StreamReader {
            receiver,
            current: io::Cursor::new(Vec::new()),
            ended: false,
            failure: None,
            cancel: cancel.clone(),
            local,
            _worker: worker.clone(),
        };
        drop(stream);
        done_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        assert!(!cancel.is_cancelled());
    }

    #[test]
    fn terminal_errors_and_missing_eof_remain_failures_after_retries() {
        for explicit_error in [false, true] {
            let (sender, receiver) = mpsc::sync_channel(2);
            sender.send(StreamMessage::Bytes(b"abc".to_vec())).unwrap();
            if explicit_error {
                sender
                    .send(StreamMessage::Error(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "fixture integrity failure",
                    )))
                    .unwrap();
            }
            drop(sender);
            let mut stream = StreamReader {
                receiver,
                current: io::Cursor::new(Vec::new()),
                ended: false,
                failure: None,
                cancel: Cancellation::default(),
                local: Cancellation::default(),
                _worker: Worker::start().unwrap(),
            };
            let mut bytes = [0; 3];
            stream.read_exact(&mut bytes).unwrap();
            assert_eq!(&bytes, b"abc");
            let initial = stream.read(&mut [0; 1]).unwrap_err();
            for _ in 0..3 {
                let repeated = stream.read(&mut [0; 1]).unwrap_err();
                assert_eq!(initial.kind(), repeated.kind());
                assert_eq!(initial.to_string(), repeated.to_string());
            }
        }
    }
}
