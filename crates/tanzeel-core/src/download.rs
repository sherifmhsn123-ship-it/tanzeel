use std::net::IpAddr;

use thiserror::Error;
use url::Url;

/// Maximum accepted length of a download URL.
pub const MAX_URL_LEN: usize = 8192;

/// Maximum accepted length of a sanitized file name, in characters.
pub const MAX_FILE_NAME_LEN: usize = 128;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UrlValidationError {
    #[error("url is empty")]
    Empty,
    #[error("url is longer than {MAX_URL_LEN} characters")]
    TooLong,
    #[error("url has leading or trailing whitespace")]
    Whitespace,
    #[error("url is not valid")]
    Invalid,
    #[error("only http and https urls are supported")]
    UnsupportedScheme,
    #[error("url has no host")]
    MissingHost,
    #[error("url credentials are not allowed")]
    Credentials,
    #[error("local and private network hosts are blocked by default")]
    LocalOrPrivateHost,
}

/// Parses and validates a user-supplied download URL.
///
/// Private and loopback hosts are rejected unless `allow_private_network` is
/// explicitly enabled. That escape hatch exists for tests and a future advanced
/// setting; the default path must stay closed.
pub fn validate_download_url(
    input: &str,
    allow_private_network: bool,
) -> Result<Url, UrlValidationError> {
    if input.is_empty() {
        return Err(UrlValidationError::Empty);
    }
    if input.chars().count() > MAX_URL_LEN {
        return Err(UrlValidationError::TooLong);
    }
    if input.trim() != input {
        return Err(UrlValidationError::Whitespace);
    }

    let url = Url::parse(input).map_err(|_| UrlValidationError::Invalid)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(UrlValidationError::UnsupportedScheme);
    }
    if url.host_str().is_none() {
        return Err(UrlValidationError::MissingHost);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlValidationError::Credentials);
    }
    if !allow_private_network && url.host_str().is_some_and(is_private_or_local_host) {
        return Err(UrlValidationError::LocalOrPrivateHost);
    }

    Ok(url)
}

/// Returns true for localhost, loopback, private, link-local, and other
/// non-public IP literals. Hostnames are not resolved here.
pub fn is_private_or_local_host(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let host = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost") {
        return true;
    }

    host.parse::<IpAddr>()
        .map(is_private_or_local_ip)
        .unwrap_or(false)
}

pub fn is_private_or_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
                // Carrier-grade NAT.
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                // IETF protocol assignments.
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                // Benchmarking.
                || (octets[0] == 198 && (18..=19).contains(&octets[1]))
                // Reserved.
                || octets[0] >= 240
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_private_or_local_ip(IpAddr::V4(mapped));
            }

            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast()
        }
    }
}

/// Produces a single safe file name component using Windows rules.
///
/// Path separators are discarded by taking the final component, forbidden
/// characters are replaced, reserved device names are prefixed, and trailing
/// dots/spaces are removed. The result never contains a path separator.
pub fn safe_filename(input: &str, fallback: &str) -> String {
    let fallback = safe_fallback(fallback);
    let mut name = sanitize_component(input);

    if name.is_empty() || name == "." || name == ".." {
        name = fallback;
    }
    if is_reserved_windows_name(&name) {
        name = format!("_{name}");
    }

    name = name.chars().take(MAX_FILE_NAME_LEN).collect();
    name = name.trim_end_matches([' ', '.']).to_string();
    if name.is_empty() {
        return "download".to_string();
    }

    name
}

fn safe_fallback(fallback: &str) -> String {
    let mut value = sanitize_component(fallback);
    if value.is_empty() || value == "." || value == ".." {
        value = "download".to_string();
    }
    if is_reserved_windows_name(&value) {
        value = format!("_{value}");
    }
    value
}

fn sanitize_component(input: &str) -> String {
    let component = input.rsplit(['/', '\\']).next().unwrap_or("");

    let mut sanitized = String::with_capacity(component.len());
    for character in component.chars() {
        let forbidden =
            character.is_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*');
        sanitized.push(if forbidden { '_' } else { character });
    }

    sanitized.trim_end_matches([' ', '.']).to_string()
}

fn is_reserved_windows_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();

    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CONIN$",
        "CONOUT$",
    ];

    RESERVED.contains(&stem.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https() {
        assert!(validate_download_url("https://example.com/file.zip", false).is_ok());
        assert!(validate_download_url("http://example.com/file.zip", false).is_ok());
    }

    #[test]
    fn accepts_non_standard_ports() {
        let url = validate_download_url("https://example.com:8443/file.zip", false)
            .expect("url is valid");
        assert_eq!(url.port(), Some(8443));
    }

    #[test]
    fn rejects_unsupported_and_empty_urls() {
        for url in [
            "",
            "   ",
            "file:///C:/Windows/System32/evil.exe",
            "javascript:alert(1)",
            "data:text/plain,hello",
            "ftp://example.com/file.zip",
        ] {
            assert!(
                validate_download_url(url, false).is_err(),
                "{url} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_credentials_and_overlong_urls() {
        assert_eq!(
            validate_download_url("https://user:secret@example.com/file.zip", false),
            Err(UrlValidationError::Credentials)
        );
        assert_eq!(
            validate_download_url("https://user@example.com/file.zip", false),
            Err(UrlValidationError::Credentials)
        );

        let long = format!("https://example.com/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(
            validate_download_url(&long, false),
            Err(UrlValidationError::TooLong)
        );
    }

    #[test]
    fn rejects_whitespace_and_invalid_urls() {
        assert_eq!(
            validate_download_url(" https://example.com/file.zip", false),
            Err(UrlValidationError::Whitespace)
        );
        assert_eq!(
            validate_download_url("not a url", false),
            Err(UrlValidationError::Invalid)
        );
    }

    #[test]
    fn rejects_local_and_private_hosts_by_default() {
        for url in [
            "http://localhost/file.zip",
            "http://localhost./file.zip",
            "http://app.localhost/file.zip",
            "http://127.0.0.1/file.zip",
            "http://127.10.20.30/file.zip",
            "http://10.0.0.1/file.zip",
            "http://172.16.0.1/file.zip",
            "http://192.168.1.1/file.zip",
            "http://169.254.1.1/file.zip",
            "http://100.64.0.1/file.zip",
            "http://0.0.0.0/file.zip",
            "http://[::1]/file.zip",
            "http://[fc00::1]/file.zip",
            "http://[fe80::1]/file.zip",
            "http://[::ffff:127.0.0.1]/file.zip",
        ] {
            assert_eq!(
                validate_download_url(url, false),
                Err(UrlValidationError::LocalOrPrivateHost),
                "{url} should be blocked"
            );
        }
    }

    #[test]
    fn allows_public_ip_literals() {
        assert!(validate_download_url("http://8.8.8.8/file.zip", false).is_ok());
        assert!(validate_download_url("http://[2001:4860:4860::8888]/file.zip", false).is_ok());
    }

    #[test]
    fn allows_private_network_only_when_explicitly_requested() {
        assert!(validate_download_url("http://127.0.0.1:8080/file.zip", true).is_ok());
        assert!(validate_download_url("http://localhost:8080/file.zip", true).is_ok());
    }

    #[test]
    fn sanitize_path_traversal() {
        assert_eq!(
            safe_filename(r"..\..\Windows\System32\evil.exe", "download"),
            "evil.exe"
        );
        assert_eq!(
            safe_filename("../../home/user/.bashrc", "download"),
            ".bashrc"
        );
    }

    #[test]
    fn safe_filename_replaces_windows_forbidden_characters() {
        assert_eq!(
            safe_filename(r#"a<b>c:d"e|f?g*h.txt"#, "download"),
            "a_b_c_d_e_f_g_h.txt"
        );
        assert_eq!(
            safe_filename("line\nbreak.txt", "download"),
            "line_break.txt"
        );
    }

    #[test]
    fn sanitize_windows_reserved_names() {
        for name in ["CON", "con.txt", "PRN", "AUX", "NUL", "COM1", "lpt9.log"] {
            assert!(
                safe_filename(name, "download").starts_with('_'),
                "{name} should be prefixed"
            );
        }
    }

    #[test]
    fn safe_filename_preserves_arabic_names() {
        assert_eq!(
            safe_filename("تقرير-التنزيل النهائي.pdf", "download"),
            "تقرير-التنزيل النهائي.pdf"
        );
    }

    #[test]
    fn safe_filename_removes_trailing_dots_and_spaces() {
        assert_eq!(safe_filename("report.  ", "download"), "report");
        assert_eq!(safe_filename("report.", "download"), "report");
    }

    #[test]
    fn safe_filename_limits_length_without_splitting_utf8() {
        let name = safe_filename(&"م".repeat(MAX_FILE_NAME_LEN + 20), "download");
        assert_eq!(name.chars().count(), MAX_FILE_NAME_LEN);
        assert!(name.is_char_boundary(name.len()));
    }

    #[test]
    fn safe_filename_uses_fallback_for_empty_and_dot_names() {
        assert_eq!(safe_filename("", "download"), "download");
        assert_eq!(safe_filename(".", "download"), "download");
        assert_eq!(safe_filename("..", "download"), "download");
        assert_eq!(safe_filename("   ", "download"), "download");
    }

    #[test]
    fn safe_filename_never_contains_a_path_separator() {
        for input in ["a/b/c", r"a\b\c", r"..\..\Windows\evil", "normal.txt"] {
            let name = safe_filename(input, "download");
            assert!(!name.contains('/'));
            assert!(!name.contains('\\'));
        }
    }
}
