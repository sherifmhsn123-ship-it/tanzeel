use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tanzeel_download::{
    download, DownloadControl, DownloadError, DownloadOptions, DownloadOutcome, DownloadRequest,
};

#[derive(Debug)]
struct Request {
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    path: String,
    range_start: Option<u64>,
}

struct Response {
    status: u16,
    body: Vec<u8>,
    support_range: bool,
    interrupt_after: Option<usize>,
    delay: Option<Duration>,
    headers: Vec<(String, String)>,
    etag: String,
}

impl Response {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            support_range: true,
            interrupt_after: None,
            delay: None,
            headers: Vec::new(),
            etag: "\"v1\"".to_string(),
        }
    }

    fn no_range(body: impl Into<Vec<u8>>) -> Self {
        Self {
            support_range: false,
            ..Self::ok(body)
        }
    }

    fn status(status: u16) -> Self {
        Self {
            status,
            body: Vec::new(),
            support_range: false,
            interrupt_after: None,
            delay: None,
            headers: Vec::new(),
            etag: "\"v1\"".to_string(),
        }
    }

    fn with_interrupt(mut self, bytes: usize) -> Self {
        self.interrupt_after = Some(bytes);
        self
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    fn with_etag(mut self, etag: &str) -> Self {
        self.etag = etag.to_string();
        self
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

struct TestServer {
    base_url: String,
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TestServer {
    fn start<F>(handler: F) -> Self
    where
        F: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let handler = Arc::new(handler);

        let handle = thread::spawn(move || {
            for incoming in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = incoming else {
                    continue;
                };
                let handler = handler.clone();
                thread::spawn(move || {
                    let _ = serve(stream, &*handler);
                });
            }
        });

        Self {
            base_url: format!("http://{addr}"),
            addr,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn serve(mut stream: TcpStream, handler: &dyn Fn(&Request) -> Response) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let mut range_start = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("range") {
                range_start = parse_range(value.trim());
            }
        }
    }

    let request = Request {
        method,
        path,
        range_start,
    };
    let response = handler(&request);
    write_response(&mut stream, &request, response)
}

fn write_response(stream: &mut TcpStream, request: &Request, response: Response) -> io::Result<()> {
    let Response {
        mut status,
        mut body,
        support_range,
        interrupt_after,
        delay,
        mut headers,
        etag,
    } = response;

    if status == 200 && support_range {
        if let Some(start) = request.range_start {
            let total = body.len();
            if start >= total as u64 {
                status = 416;
                headers.push(("Content-Range".to_string(), format!("bytes */{total}")));
                body.clear();
            } else {
                status = 206;
                headers.push((
                    "Content-Range".to_string(),
                    format!("bytes {}-{}/{total}", start, total - 1),
                ));
                body = body[start as usize..].to_vec();
            }
        }
    }

    if status == 200 && support_range {
        headers.push(("Accept-Ranges".to_string(), "bytes".to_string()));
    }
    if status == 200 || status == 206 {
        headers.push(("ETag".to_string(), etag));
        headers.push((
            "Last-Modified".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        ));
    }

    let content_length = body.len();
    let mut output = Vec::new();
    write!(&mut output, "HTTP/1.1 {} {}\r\n", status, reason(status))?;
    write!(&mut output, "Content-Length: {content_length}\r\n")?;
    write!(&mut output, "Connection: close\r\n")?;
    for (name, value) in headers {
        write!(&mut output, "{name}: {value}\r\n")?;
    }
    output.extend_from_slice(b"\r\n");
    stream.write_all(&output)?;

    if status == 200 || status == 206 {
        if let Some(interrupt) = interrupt_after {
            let sent = interrupt.min(body.len());
            stream.write_all(&body[..sent])?;
            stream.flush()?;
            return Ok(());
        }

        if let Some(delay) = delay {
            for chunk in body.chunks(4096) {
                stream.write_all(chunk)?;
                stream.flush()?;
                thread::sleep(delay);
            }
            return Ok(());
        }
    }

    stream.write_all(&body)?;
    stream.flush()?;
    Ok(())
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        302 => "Found",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn parse_range(value: &str) -> Option<u64> {
    let rest = value.strip_prefix("bytes=")?;
    let start = rest.split('-').next()?;
    start.trim().parse().ok()
}

fn request(server: &TestServer, path: &str, directory: &Path) -> DownloadRequest {
    DownloadRequest {
        url: server.url(path),
        destination_dir: directory.to_path_buf(),
        job_id: "job1".to_string(),
        file_name: None,
        expected_sha256: None,
    }
}

fn options() -> DownloadOptions {
    DownloadOptions {
        allow_private_network: true,
        ..DownloadOptions::default()
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn assert_no_partials(directory: &Path) {
    for entry in fs::read_dir(directory).expect("read temp directory") {
        let entry = entry.expect("read temp entry");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.ends_with(".part") && !name.ends_with(".part.meta"),
            "partial file was not cleaned: {name}"
        );
    }
}

fn run_download(server: &TestServer, directory: &Path) -> Result<DownloadOutcome, DownloadError> {
    download(
        request(server, "/file.bin", directory),
        options(),
        DownloadControl::new(),
        |_| {},
    )
}

#[test]
fn download_success() {
    let body = b"hello world\n".repeat(4096);
    let expected = sha256_hex(&body);
    let server = TestServer::start({
        let body = body.clone();
        move |_| Response::ok(body.clone())
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("download succeeds");

    assert_eq!(outcome.bytes, body.len() as u64);
    assert_eq!(outcome.sha256, expected);
    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn download_resume_after_interruption() {
    let body: Vec<u8> = (0..64 * 1024).map(|index| (index % 251) as u8).collect();
    let first = Arc::new(AtomicBool::new(true));
    let server = TestServer::start({
        let body = body.clone();
        let first = first.clone();
        move |_| {
            if first.swap(false, Ordering::SeqCst) {
                Response::ok(body.clone()).with_interrupt(4096)
            } else {
                Response::ok(body.clone())
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("resume succeeds");

    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn download_restart_when_no_range() {
    let body: Vec<u8> = (0..64 * 1024).map(|index| (index % 239) as u8).collect();
    let first = Arc::new(AtomicBool::new(true));
    let server = TestServer::start({
        let body = body.clone();
        let first = first.clone();
        move |_| {
            if first.swap(false, Ordering::SeqCst) {
                Response::no_range(body.clone()).with_interrupt(4096)
            } else {
                Response::no_range(body.clone())
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("restart succeeds");

    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn download_cancel_cleans_partial() {
    let body = vec![7_u8; 512 * 1024];
    let server = TestServer::start({
        let body = body.clone();
        move |_| Response::ok(body.clone()).with_delay(Duration::from_millis(10))
    });
    let directory = tempfile::tempdir().expect("temp directory");
    let control = DownloadControl::new();
    let control_thread = control.clone();
    let pending = request(&server, "/file.bin", directory.path());

    let handle = thread::spawn(move || download(pending, options(), control_thread, |_| {}));
    thread::sleep(Duration::from_millis(150));
    control.cancel();
    let result = handle.join().expect("download thread panicked");

    assert!(matches!(result, Err(DownloadError::Cancelled)));
    assert_no_partials(directory.path());
}

#[test]
fn download_404_returns_not_found() {
    let server = TestServer::start(|_| Response::status(404));
    let directory = tempfile::tempdir().expect("temp directory");

    let result = run_download(&server, directory.path());

    assert!(matches!(result, Err(DownloadError::NotFound)));
    assert_no_partials(directory.path());
}

#[test]
fn download_500_retries() {
    let body = b"retry body".repeat(1024);
    let attempts = Arc::new(AtomicUsize::new(0));
    let server = TestServer::start({
        let body = body.clone();
        let attempts = attempts.clone();
        move |_| {
            if attempts.fetch_add(1, Ordering::SeqCst) < 2 {
                Response::status(500)
            } else {
                Response::ok(body.clone())
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("retry succeeds");

    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn download_429_respects_retry_after() {
    let body = b"rate limited body".repeat(128);
    let attempts = Arc::new(AtomicUsize::new(0));
    let server = TestServer::start({
        let body = body.clone();
        let attempts = attempts.clone();
        move |_| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                Response::status(429).header("Retry-After", "0")
            } else {
                Response::ok(body.clone())
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("rate limit retry succeeds");

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn progress_events_are_throttled() {
    let body = vec![3_u8; 2 * 1024 * 1024];
    let server = TestServer::start({
        let body = body.clone();
        move |_| Response::ok(body.clone())
    });
    let directory = tempfile::tempdir().expect("temp directory");
    let events = Arc::new(AtomicUsize::new(0));

    let outcome = download(
        request(&server, "/file.bin", directory.path()),
        options(),
        DownloadControl::new(),
        {
            let events = events.clone();
            move |_| {
                events.fetch_add(1, Ordering::SeqCst);
            }
        },
    )
    .expect("download succeeds");

    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert!(
        events.load(Ordering::SeqCst) <= 10,
        "progress callback fired too often"
    );
}

#[test]
fn etag_change_restarts_safely() {
    let body: Vec<u8> = (0..64 * 1024).map(|index| (index % 197) as u8).collect();
    let requests = Arc::new(AtomicUsize::new(0));
    let server = TestServer::start({
        let body = body.clone();
        let requests = requests.clone();
        move |_| {
            if requests.fetch_add(1, Ordering::SeqCst) == 0 {
                Response::ok(body.clone()).with_interrupt(4096)
            } else {
                Response::ok(body.clone()).with_etag("\"v2\"")
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("safe restart succeeds");

    assert!(requests.load(Ordering::SeqCst) >= 2);
    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
    assert_no_partials(directory.path());
}

#[test]
fn content_disposition_names_the_final_file() {
    let server = TestServer::start(|_| {
        Response::ok(b"data".to_vec())
            .header("Content-Disposition", "attachment; filename=\"تقرير.pdf\"")
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("download succeeds");

    assert_eq!(
        outcome
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name"),
        "تقرير.pdf"
    );
}

#[test]
fn content_disposition_traversal_is_sanitized() {
    let server = TestServer::start(|_| {
        Response::ok(b"data".to_vec()).header(
            "Content-Disposition",
            "attachment; filename=\"..\\..\\evil.exe\"",
        )
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("download succeeds");

    assert_eq!(
        outcome
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name"),
        "evil.exe"
    );
}

#[test]
fn zero_byte_file_is_downloaded() {
    let server = TestServer::start(|_| Response::ok(Vec::new()));
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = run_download(&server, directory.path()).expect("download succeeds");

    assert_eq!(outcome.bytes, 0);
    assert_eq!(fs::metadata(&outcome.path).expect("file exists").len(), 0);
    assert_eq!(outcome.sha256, sha256_hex(&[]));
}

#[test]
fn redirect_to_http_is_followed() {
    let body = b"redirected".to_vec();
    let server = TestServer::start({
        let body = body.clone();
        move |request| {
            if request.path == "/start" {
                Response::status(302).header("Location", "/file.bin")
            } else {
                Response::ok(body.clone())
            }
        }
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let outcome = download(
        request(&server, "/start", directory.path()),
        options(),
        DownloadControl::new(),
        |_| {},
    )
    .expect("redirect succeeds");

    assert_eq!(fs::read(&outcome.path).expect("read downloaded file"), body);
}

#[test]
fn redirect_with_credentials_is_rejected() {
    let server = TestServer::start(|_| {
        Response::status(302).header("Location", "http://user:secret@example.com/file.bin")
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let result = run_download(&server, directory.path());

    assert!(matches!(result, Err(DownloadError::RedirectRejected)));
    assert_no_partials(directory.path());
}

#[test]
fn redirect_to_file_scheme_rejected() {
    let server = TestServer::start(|_| {
        Response::status(302).header("Location", "file:///C:/Windows/System32/evil.exe")
    });
    let directory = tempfile::tempdir().expect("temp directory");

    let result = run_download(&server, directory.path());

    assert!(matches!(result, Err(DownloadError::RedirectRejected)));
    assert_no_partials(directory.path());
}

#[test]
fn private_ip_rejected_by_default() {
    let server = TestServer::start(|_| Response::ok(b"data".to_vec()));
    let directory = tempfile::tempdir().expect("temp directory");

    let result = download(
        request(&server, "/file.bin", directory.path()),
        DownloadOptions::default(),
        DownloadControl::new(),
        |_| {},
    );

    assert!(matches!(result, Err(DownloadError::InvalidUrl(_))));
    assert_no_partials(directory.path());
}
