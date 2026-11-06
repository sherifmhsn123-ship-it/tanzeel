use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use percent_encoding::percent_decode_str;
use reqwest::blocking::{Client, Response};
use reqwest::header::{
    HeaderMap, HeaderName, ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_RANGE, ETAG, IF_RANGE,
    LAST_MODIFIED, RANGE, RETRY_AFTER,
};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tanzeel_core::{
    is_private_or_local_host, is_private_or_local_ip, safe_filename, validate_download_url,
};
use thiserror::Error;

/// Maximum number of redirects followed by the HTTP client.
pub const MAX_REDIRECTS: usize = 5;
/// Default number of retries after the first attempt.
pub const DEFAULT_MAX_RETRIES: u32 = 3;
/// Minimum interval between progress callbacks.
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub destination_dir: PathBuf,
    pub job_id: String,
    /// Optional final file name. When absent, a safe name is derived from the
    /// URL and `Content-Disposition`.
    pub file_name: Option<String>,
    /// Optional lowercase or uppercase SHA-256 hex digest.
    pub expected_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Tests and a future explicit advanced setting may allow loopback/private
    /// hosts. Production defaults to false.
    pub allow_private_network: bool,
    pub max_retries: u32,
    pub connect_timeout: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            allow_private_network: false,
            max_retries: DEFAULT_MAX_RETRIES,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DownloadControl {
    cancelled: Arc<AtomicBool>,
}

impl DownloadControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub speed_bytes_per_second: u64,
    pub eta_seconds: Option<u64>,
    pub active_segments: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid url: {0}")]
    InvalidUrl(#[from] tanzeel_core::UrlValidationError),
    #[error("invalid job id")]
    InvalidJobId,
    #[error("destination directory does not exist or is not a directory")]
    InvalidDestination,
    #[error("host resolves to a local or private address")]
    PrivateNetwork,
    #[error("could not resolve host: {0}")]
    Dns(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("network read error: {0}")]
    Read(#[source] io::Error),
    #[error("http status {0}")]
    Status(u16),
    #[error("the file was not found")]
    NotFound,
    #[error("the server asked us to slow down")]
    RateLimited { retry_after: Option<Duration> },
    #[error("server error {0}")]
    ServerError(u16),
    #[error("redirect rejected")]
    RedirectRejected,
    #[error("a safe restart is required")]
    RestartRequired,
    #[error("response ended early: expected {expected} bytes, received {actual}")]
    Incomplete { expected: u64, actual: u64 },
    #[error("the disk is full")]
    DiskFull,
    #[error("permission denied")]
    Permission,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("cancelled")]
    Cancelled,
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl DownloadError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_)
            | Self::Read(_)
            | Self::Dns(_)
            | Self::ServerError(_)
            | Self::RateLimited { .. }
            | Self::Incomplete { .. } => true,
            Self::Io(error) => matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ),
            _ => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => *retry_after,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PartialMetadata {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    total: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentRange {
    start: u64,
    end: u64,
    total: Option<u64>,
}

/// Downloads one direct HTTP/HTTPS file with safe resume and cancellation.
///
/// The function is blocking. Callers should run it on a worker thread.
pub fn download<F>(
    request: DownloadRequest,
    options: DownloadOptions,
    control: DownloadControl,
    mut on_progress: F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let url = validate_download_url(&request.url, options.allow_private_network)?;
    if !is_valid_job_id(&request.job_id) {
        return Err(DownloadError::InvalidJobId);
    }
    if !options.allow_private_network {
        check_public_host(&url)?;
    }

    let directory = request.destination_dir.canonicalize().map_err(io_error)?;
    if !directory.is_dir() {
        return Err(DownloadError::InvalidDestination);
    }

    let temp_name = request
        .file_name
        .as_deref()
        .map(|name| safe_filename(name, "download"))
        .unwrap_or_else(|| filename_from_url(&url));
    let part_path = directory.join(format!("{temp_name}.{}.part", request.job_id));
    let meta_path = directory.join(format!("{temp_name}.{}.part.meta", request.job_id));

    let mut retries = 0_u32;
    let mut restarts = 0_u8;
    let mut force_restart = false;

    loop {
        if control.is_cancelled() {
            cleanup(&part_path, &meta_path);
            return Err(DownloadError::Cancelled);
        }

        if force_restart {
            let _ = fs::remove_file(&part_path);
            let _ = fs::remove_file(&meta_path);
            force_restart = false;
            retries = 0;
        }

        match attempt_download(
            &url,
            &directory,
            &temp_name,
            &request,
            &options,
            &control,
            &part_path,
            &meta_path,
            &mut on_progress,
        ) {
            Ok(outcome) => return Ok(outcome),
            Err(DownloadError::RestartRequired) if restarts < 1 => {
                restarts += 1;
                force_restart = true;
                continue;
            }
            Err(error) if error.is_retryable() && retries < options.max_retries => {
                retries += 1;
                let delay = error.retry_after().unwrap_or_else(|| backoff(retries));
                if !sleep_with_cancel(&control, delay) {
                    cleanup(&part_path, &meta_path);
                    return Err(DownloadError::Cancelled);
                }
                continue;
            }
            Err(DownloadError::Cancelled) => {
                cleanup(&part_path, &meta_path);
                return Err(DownloadError::Cancelled);
            }
            Err(error) => return Err(error),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn attempt_download<F>(
    url: &Url,
    directory: &Path,
    temp_name: &str,
    request: &DownloadRequest,
    options: &DownloadOptions,
    control: &DownloadControl,
    part_path: &Path,
    meta_path: &Path,
    on_progress: &mut F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let mut metadata = read_metadata(meta_path)?;
    let mut existing = fs::metadata(part_path).map(|meta| meta.len()).unwrap_or(0);

    if existing > 0 && metadata.is_none() {
        remove_partial(part_path, meta_path);
        existing = 0;
    }

    if let Some(meta) = &metadata {
        if meta.url != url.as_str() {
            remove_partial(part_path, meta_path);
            metadata = None;
            existing = 0;
        }
    }

    let client = build_client(options)?;
    let response = send_request(&client, url, existing, metadata.as_ref())?;
    let status = response.status();
    let headers = response.headers().clone();

    if status != StatusCode::OK && status != StatusCode::PARTIAL_CONTENT {
        return Err(classify_status(status, &headers));
    }

    let content_length = response.content_length();
    let content_range = parse_content_range(&headers);
    let etag = header_string(&headers, ETAG);
    let last_modified = header_string(&headers, LAST_MODIFIED);

    let mut reset = false;
    let total;

    if status == StatusCode::PARTIAL_CONTENT {
        let range = content_range.ok_or_else(|| {
            DownloadError::InvalidResponse("206 response has no valid Content-Range".into())
        })?;

        if range.start != existing || range.end < range.start {
            return Err(DownloadError::RestartRequired);
        }
        if let Some(total) = range.total {
            if range.end.saturating_add(1) != total {
                return Err(DownloadError::RestartRequired);
            }
        }
        if let Some(meta) = &metadata {
            if meta.total.is_some() && range.total.is_some() && meta.total != range.total {
                return Err(DownloadError::RestartRequired);
            }
        }

        total = range
            .total
            .or_else(|| metadata.as_ref().and_then(|meta| meta.total))
            .or_else(|| content_length.map(|length| range.start + length));
    } else {
        // The server ignored Range, so the only safe move is to restart.
        reset = existing > 0;
        total = content_length;
    }

    if existing > 0 {
        if let Some(meta) = &metadata {
            if validator_changed(meta, etag.as_deref(), last_modified.as_deref()) {
                return Err(DownloadError::RestartRequired);
            }
        }
    }

    if let Some(meta) = &metadata {
        if meta.total.is_some() && total.is_some() && meta.total != total {
            return Err(DownloadError::RestartRequired);
        }
    }

    let metadata_to_store = PartialMetadata {
        url: url.to_string(),
        etag,
        last_modified,
        total,
    };
    write_metadata(meta_path, &metadata_to_store)?;

    let (mut file, mut hasher, mut bytes_done) = if reset || existing == 0 {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(part_path)
            .map_err(io_error)?;
        (file, Sha256::new(), 0)
    } else {
        let file = OpenOptions::new()
            .append(true)
            .open(part_path)
            .map_err(io_error)?;
        let mut hasher = Sha256::new();
        hash_file(part_path, &mut hasher)?;
        (file, hasher, existing)
    };

    let started_at = Instant::now();
    let started_bytes = bytes_done;
    let mut last_emit = Instant::now();
    let mut last_emit_bytes = bytes_done;
    let mut last_emit_time = Instant::now();
    emit_progress(on_progress, bytes_done, total, 0, None);
    let mut response = response;
    let mut buffer = [0_u8; BUFFER_SIZE];

    loop {
        if control.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        let read = response.read(&mut buffer).map_err(DownloadError::Read)?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read]).map_err(io_error)?;
        hasher.update(&buffer[..read]);
        bytes_done += read as u64;

        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            let elapsed = last_emit_time.elapsed().as_secs_f64().max(0.001);
            let speed = ((bytes_done - last_emit_bytes) as f64 / elapsed).round() as u64;
            let eta = total.map(|total| eta_seconds(total, bytes_done, speed));
            emit_progress(on_progress, bytes_done, total, speed, eta);
            last_emit = Instant::now();
            last_emit_bytes = bytes_done;
            last_emit_time = Instant::now();
        }
    }

    file.flush().map_err(io_error)?;
    file.sync_all().map_err(io_error)?;

    if let Some(total) = total {
        if bytes_done != total {
            return Err(DownloadError::Incomplete {
                expected: total,
                actual: bytes_done,
            });
        }
    }

    let actual_sha256 = hex::encode(hasher.finalize());
    if let Some(expected) = &request.expected_sha256 {
        if !actual_sha256.eq_ignore_ascii_case(expected.trim()) {
            drop(file);
            remove_partial(part_path, meta_path);
            return Err(DownloadError::ChecksumMismatch {
                expected: expected.clone(),
                actual: actual_sha256,
            });
        }
    }

    let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
    let speed = ((bytes_done.saturating_sub(started_bytes)) as f64 / elapsed).round() as u64;
    emit_progress(on_progress, bytes_done, total, speed, Some(0));

    drop(file);

    let final_name = request
        .file_name
        .as_deref()
        .map(|name| safe_filename(name, temp_name))
        .or_else(|| filename_from_content_disposition(&headers))
        .unwrap_or_else(|| temp_name.to_string());

    let final_path = unique_destination(directory, &final_name)?;
    fs::rename(part_path, &final_path).map_err(io_error)?;
    let _ = fs::remove_file(meta_path);

    Ok(DownloadOutcome {
        path: final_path,
        bytes: bytes_done,
        sha256: actual_sha256,
    })
}

fn is_https_downgrade(previous: &[Url], next: &Url) -> bool {
    previous
        .last()
        .is_some_and(|previous| previous.scheme() == "https" && next.scheme() == "http")
}

/// Rejects a URL whose host is private or resolves to a private address.
///
/// Literal IPs are already checked by `validate_download_url`; this also covers
/// hostnames. DNS answers are checked before the request and again for every
/// redirect. A malicious resolver can still change its answer between this
/// check and the actual connection (DNS rebinding); that residual risk is
/// documented and not yet mitigated.
fn check_public_host(url: &Url) -> Result<(), DownloadError> {
    let host = url
        .host_str()
        .ok_or_else(|| DownloadError::InvalidResponse("url has no host".into()))?;
    let bare = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = bare.parse::<IpAddr>() {
        return if is_private_or_local_ip(ip) {
            Err(DownloadError::PrivateNetwork)
        } else {
            Ok(())
        };
    }

    if is_private_or_local_host(host) {
        return Err(DownloadError::PrivateNetwork);
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| DownloadError::InvalidResponse("url has no known default port".into()))?;
    let addresses = (bare, port)
        .to_socket_addrs()
        .map_err(|error| DownloadError::Dns(error.to_string()))?;

    let mut resolved = false;
    for address in addresses {
        resolved = true;
        if is_private_or_local_ip(address.ip()) {
            return Err(DownloadError::PrivateNetwork);
        }
    }

    if resolved {
        Ok(())
    } else {
        Err(DownloadError::Dns("host resolved to no addresses".into()))
    }
}

fn build_client(options: &DownloadOptions) -> Result<Client, DownloadError> {
    let allow_private = options.allow_private_network;
    let redirect = Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }

        let url = attempt.url();
        if !matches!(url.scheme(), "http" | "https") {
            return attempt.error("unsupported redirect scheme");
        }
        if is_https_downgrade(attempt.previous(), url) {
            return attempt.error("redirect downgrades https to http");
        }
        if !url.username().is_empty() || url.password().is_some() {
            return attempt.error("redirect credentials are not allowed");
        }
        if !allow_private {
            if let Err(error) = check_public_host(url) {
                return attempt.error(error);
            }
        }

        attempt.follow()
    });

    Client::builder()
        .redirect(redirect)
        .connect_timeout(options.connect_timeout)
        .no_proxy()
        .user_agent("Tanzeel/0.1")
        .build()
        .map_err(|error| {
            if error.is_redirect() {
                DownloadError::RedirectRejected
            } else {
                DownloadError::Network(error)
            }
        })
}

fn send_request(
    client: &Client,
    url: &Url,
    existing: u64,
    metadata: Option<&PartialMetadata>,
) -> Result<Response, DownloadError> {
    let mut request = client.get(url.clone()).header(ACCEPT_ENCODING, "identity");

    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
        if let Some(value) = metadata.and_then(if_range_value) {
            request = request.header(IF_RANGE, value);
        }
    }

    request.send().map_err(|error| {
        if error.is_redirect() {
            DownloadError::RedirectRejected
        } else {
            DownloadError::Network(error)
        }
    })
}

fn if_range_value(metadata: &PartialMetadata) -> Option<String> {
    match &metadata.etag {
        Some(etag) if !etag.starts_with("W/") => Some(etag.clone()),
        _ => metadata.last_modified.clone(),
    }
}

fn classify_status(status: StatusCode, headers: &HeaderMap) -> DownloadError {
    match status.as_u16() {
        200..=299 => DownloadError::Status(status.as_u16()),
        300..=399 => DownloadError::RedirectRejected,
        404 | 410 => DownloadError::NotFound,
        416 => DownloadError::RestartRequired,
        429 | 503 => DownloadError::RateLimited {
            retry_after: parse_retry_after(headers),
        },
        500..=599 => DownloadError::ServerError(status.as_u16()),
        code => DownloadError::Status(code),
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn parse_content_range(headers: &HeaderMap) -> Option<ContentRange> {
    let value = headers.get(CONTENT_RANGE)?.to_str().ok()?;
    let rest = value.strip_prefix("bytes ")?;
    let (range, total) = rest.split_once('/')?;
    let (start, end) = range.split_once('-')?;

    Some(ContentRange {
        start: start.parse().ok()?,
        end: end.parse().ok()?,
        total: if total == "*" {
            None
        } else {
            Some(total.parse().ok()?)
        },
    })
}

fn header_string(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn filename_from_url(url: &Url) -> String {
    let segment = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("");
    let decoded = percent_decode_str(segment).decode_utf8_lossy();
    safe_filename(&decoded, "download")
}

fn filename_from_content_disposition(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(CONTENT_DISPOSITION)?;
    let value = String::from_utf8_lossy(value.as_bytes());
    let mut plain = None;
    let mut extended = None;

    for part in value.split(';') {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();

        if name == "filename*" {
            extended = Some(value);
        } else if name == "filename" {
            plain = Some(value);
        }
    }

    let raw = extended
        .and_then(parse_extended_filename)
        .or_else(|| plain.map(unquote_filename))?;

    Some(safe_filename(&raw, "download"))
}

fn parse_extended_filename(value: &str) -> Option<String> {
    let (_, encoded) = value.split_once("''")?;
    Some(percent_decode_str(encoded).decode_utf8_lossy().into_owned())
}

fn unquote_filename(value: &str) -> String {
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value);
    value.replace("\\\"", "\"")
}

fn unique_destination(directory: &Path, name: &str) -> Result<PathBuf, DownloadError> {
    let name = safe_filename(name, "download");
    let candidate = directory.join(&name);
    if !candidate.exists() {
        return Ok(candidate);
    }

    let path = Path::new(&name);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("download");
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();

    for index in 1..=9999 {
        let candidate = directory.join(format!("{stem} ({index}){extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(DownloadError::Internal(
        "could not find a free destination name".into(),
    ))
}

fn read_metadata(path: &Path) -> Result<Option<PartialMetadata>, DownloadError> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<PartialMetadata>(&bytes) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(_) => Ok(None),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

fn write_metadata(path: &Path, metadata: &PartialMetadata) -> Result<(), DownloadError> {
    let bytes = serde_json::to_vec(metadata)
        .map_err(|error| DownloadError::Internal(format!("could not encode metadata: {error}")))?;
    fs::write(path, bytes).map_err(io_error)
}

fn hash_file(path: &Path, hasher: &mut Sha256) -> Result<(), DownloadError> {
    let mut file = File::open(path).map_err(io_error)?;
    let mut buffer = [0_u8; BUFFER_SIZE];
    loop {
        let read = file.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(())
}

fn validator_changed(
    metadata: &PartialMetadata,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> bool {
    if let Some(old) = &metadata.etag {
        match etag {
            Some(new) => old != new,
            None => true,
        }
    } else if let Some(old) = &metadata.last_modified {
        match last_modified {
            Some(new) => old != new,
            None => true,
        }
    } else {
        false
    }
}

fn emit_progress<F>(
    on_progress: &mut F,
    bytes_done: u64,
    bytes_total: Option<u64>,
    speed: u64,
    eta: Option<u64>,
) where
    F: FnMut(DownloadProgress),
{
    on_progress(DownloadProgress {
        bytes_done,
        bytes_total,
        speed_bytes_per_second: speed,
        eta_seconds: eta,
        active_segments: 1,
    });
}

fn eta_seconds(total: u64, done: u64, speed: u64) -> u64 {
    if speed == 0 {
        return 0;
    }
    total.saturating_sub(done).div_ceil(speed)
}

fn backoff(retry: u32) -> Duration {
    let base = 100_u64.saturating_mul(1_u64 << retry.saturating_sub(1).min(4));
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos()) % 50)
        .unwrap_or(0);
    Duration::from_millis(base + jitter)
}

fn sleep_with_cancel(control: &DownloadControl, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if control.is_cancelled() {
            return false;
        }
        std::thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
    }
    !control.is_cancelled()
}

fn cleanup(part_path: &Path, meta_path: &Path) {
    let _ = fs::remove_file(part_path);
    let _ = fs::remove_file(meta_path);
}

fn remove_partial(part_path: &Path, meta_path: &Path) {
    cleanup(part_path, meta_path);
}

fn is_valid_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id.len() <= 64
        && job_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn io_error(error: io::Error) -> DownloadError {
    if is_disk_full(&error) {
        DownloadError::DiskFull
    } else if error.kind() == io::ErrorKind::PermissionDenied {
        DownloadError::Permission
    } else {
        DownloadError::Io(error)
    }
}

fn is_disk_full(error: &io::Error) -> bool {
    let raw = error.raw_os_error();

    #[cfg(windows)]
    {
        // ERROR_DISK_FULL / ERROR_HANDLE_DISK_FULL.
        matches!(raw, Some(112) | Some(39))
    }

    #[cfg(unix)]
    {
        // ENOSPC, plus EDQUOT on macOS (69) and Linux (122).
        matches!(raw, Some(28) | Some(69) | Some(122))
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = raw;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_disk_full_errors() {
        #[cfg(windows)]
        let error = io::Error::from_raw_os_error(112);
        #[cfg(unix)]
        let error = io::Error::from_raw_os_error(28);

        #[cfg(any(windows, unix))]
        assert!(matches!(io_error(error), DownloadError::DiskFull));
    }

    #[test]
    fn maps_permission_errors() {
        let error = io::Error::from(io::ErrorKind::PermissionDenied);
        assert!(matches!(io_error(error), DownloadError::Permission));
    }

    #[test]
    fn rejects_unsafe_job_ids() {
        assert!(is_valid_job_id("job-1_abc"));
        assert!(!is_valid_job_id(""));
        assert!(!is_valid_job_id("../escape"));
        assert!(!is_valid_job_id("a".repeat(65).as_str()));
    }

    #[test]
    fn rejects_private_hosts_without_dns() {
        for input in [
            "http://127.0.0.1/file.bin",
            "http://10.0.0.1/file.bin",
            "http://[::1]/file.bin",
            "http://localhost/file.bin",
        ] {
            let url = Url::parse(input).expect("test url");
            assert!(
                matches!(check_public_host(&url), Err(DownloadError::PrivateNetwork)),
                "{input} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_public_ip_literals_without_dns() {
        for input in [
            "http://8.8.8.8/file.bin",
            "http://[2001:4860:4860::8888]/file.bin",
        ] {
            let url = Url::parse(input).expect("test url");
            assert!(
                check_public_host(&url).is_ok(),
                "{input} should be accepted"
            );
        }
    }

    #[test]
    fn detects_https_to_http_redirects() {
        let secure = vec![Url::parse("https://example.com/start").expect("url")];
        let downgrade = Url::parse("http://example.com/file").expect("url");
        let secure_next = Url::parse("https://example.com/file").expect("url");

        assert!(is_https_downgrade(&secure, &downgrade));
        assert!(!is_https_downgrade(&secure, &secure_next));
        assert!(!is_https_downgrade(&[], &downgrade));
    }
}
