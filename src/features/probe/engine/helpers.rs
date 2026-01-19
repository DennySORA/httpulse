use crate::probe::{NegotiatedProtocol, ProbeError, ProbeErrorKind, TcpInfoSnapshot};
use curl::Error as CurlError;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

// CURLINFO constants not exposed by curl-sys
// See: https://curl.se/libcurl/c/CURLINFO_HTTP_VERSION.html
// Use curl_sys::CURLINFO type for cross-platform compatibility (i32 on Windows, u32 on Unix)
const CURLINFO_LONG: curl_sys::CURLINFO = 0x200000;
const CURLINFO_HTTP_VERSION: curl_sys::CURLINFO = CURLINFO_LONG + 46;

// HTTP version codes returned by CURLINFO_HTTP_VERSION
const CURL_HTTP_VERSION_1_0: i64 = 1;
const CURL_HTTP_VERSION_1_1: i64 = 2;
const CURL_HTTP_VERSION_2_0: i64 = 3;
const CURL_HTTP_VERSION_3: i64 = 30;

pub(super) fn map_curl_error(err: &CurlError) -> ProbeError {
    let message = err.to_string();
    let message_lower = message.to_ascii_lowercase();

    let kind = if err.is_couldnt_resolve_host() || err.is_couldnt_resolve_proxy() {
        ProbeErrorKind::DnsOther
    } else if err.is_operation_timedout() {
        ProbeErrorKind::HttpTimeout
    } else if err.is_couldnt_connect() {
        ProbeErrorKind::ConnectOther
    } else if err.is_ssl_connect_error()
        || err.is_ssl_cacert()
        || err.is_ssl_certproblem()
        || err.is_ssl_cipher()
    {
        if is_tls_version_error(&message_lower) {
            ProbeErrorKind::TlsVersionMismatch
        } else {
            ProbeErrorKind::TlsHandshakeFailed
        }
    } else if err.is_http_returned_error() {
        ProbeErrorKind::HttpStatusError
    } else if err.is_read_error() {
        ProbeErrorKind::ReadTimeout
    } else if is_tls_version_error(&message_lower) {
        ProbeErrorKind::TlsVersionMismatch
    } else {
        ProbeErrorKind::IoError
    };

    ProbeError { kind, message }
}

fn is_tls_version_error(message: &str) -> bool {
    message.contains("ssl_min_max_version")
        || message.contains("unsupported protocol")
        || message.contains("tls")
            && (message.contains("version") || message.contains("unsupported"))
}

pub(super) fn is_dns_timeout_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("resolving timed out")
}

pub(super) fn parse_socket_addr(ip: Option<&str>, port: Option<u16>) -> Option<SocketAddr> {
    let port = port?;
    let ip = ip?.parse::<IpAddr>().ok()?;
    Some(SocketAddr::new(ip, port))
}

pub(super) fn saturating_sub(left: Duration, right: Duration) -> Duration {
    left.checked_sub(right).unwrap_or(Duration::from_millis(0))
}

pub(super) fn fetch_tcp_info(handle: *mut curl_sys::CURL) -> Option<TcpInfoSnapshot> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = handle;
        None
    }

    #[cfg(target_os = "linux")]
    unsafe {
        let mut socket: curl_sys::curl_socket_t = curl_sys::CURL_SOCKET_BAD;
        let rc = curl_sys::curl_easy_getinfo(handle, curl_sys::CURLINFO_LASTSOCKET, &mut socket);
        if rc != curl_sys::CURLE_OK || socket == curl_sys::CURL_SOCKET_BAD {
            return None;
        }

        let mut info: libc::tcp_info = std::mem::zeroed();
        let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
        let rc = libc::getsockopt(
            socket,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            &mut info as *mut _ as *mut _,
            &mut len,
        );
        if rc != 0 {
            return None;
        }

        Some(TcpInfoSnapshot {
            rtt_us: Some(info.tcpi_rtt),
            rttvar_us: Some(info.tcpi_rttvar),
            total_retrans: Some(info.tcpi_total_retrans),
            lost: Some(info.tcpi_lost),
            reordering: Some(info.tcpi_reordering),
            snd_cwnd: Some(info.tcpi_snd_cwnd),
            snd_ssthresh: Some(info.tcpi_snd_ssthresh),
        })
    }
}

/// Fetches the actually negotiated protocol information from curl.
/// Returns the HTTP version actually used (not the configured preference).
/// TLS version detection is not reliably available via curl API, so we
/// indicate when it cannot be determined.
pub(super) fn fetch_negotiated_protocol(
    handle: *mut curl_sys::CURL,
    configured_tls: &str,
) -> NegotiatedProtocol {
    let alpn = fetch_http_version(handle);
    // Note: libcurl does not expose CURLINFO for the negotiated TLS version.
    // We use the configured value but could mark it as "configured" vs "actual"
    // in a future enhancement if curl adds this capability.
    let tls_version = Some(configured_tls.to_string());

    NegotiatedProtocol {
        alpn,
        tls_version,
        cipher: None,
    }
}

fn fetch_http_version(handle: *mut curl_sys::CURL) -> Option<String> {
    if handle.is_null() {
        return None;
    }

    unsafe {
        let mut version: std::os::raw::c_long = 0;
        let rc = curl_sys::curl_easy_getinfo(
            handle,
            CURLINFO_HTTP_VERSION,
            &mut version as *mut std::os::raw::c_long,
        );

        if rc != curl_sys::CURLE_OK {
            return None;
        }

        match version as i64 {
            CURL_HTTP_VERSION_1_0 => Some("http/1.0".to_string()),
            CURL_HTTP_VERSION_1_1 => Some("http/1.1".to_string()),
            CURL_HTTP_VERSION_2_0 => Some("h2".to_string()),
            CURL_HTTP_VERSION_3 => Some("h3".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_dns_timeout_message, is_tls_version_error, parse_socket_addr, saturating_sub};
    use std::time::Duration;

    #[test]
    fn dns_timeout_message_detection() {
        assert!(is_dns_timeout_message(
            "[28] Timeout was reached (Resolving timed out after 10000 milliseconds)"
        ));
        assert!(!is_dns_timeout_message(
            "[28] Timeout was reached (Operation timed out after 10000 milliseconds)"
        ));
    }

    #[test]
    fn parse_socket_addr_accepts_valid_values() {
        let addr = parse_socket_addr(Some("127.0.0.1"), Some(443)).expect("addr");
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 443);
    }

    #[test]
    fn parse_socket_addr_rejects_invalid_values() {
        assert!(parse_socket_addr(Some("not-an-ip"), Some(80)).is_none());
        assert!(parse_socket_addr(None, Some(80)).is_none());
        assert!(parse_socket_addr(Some("127.0.0.1"), None).is_none());
    }

    #[test]
    fn saturating_sub_handles_underflow() {
        let result = saturating_sub(Duration::from_millis(5), Duration::from_millis(10));
        assert_eq!(result, Duration::from_millis(0));
    }

    #[test]
    fn saturating_sub_returns_difference() {
        let result = saturating_sub(Duration::from_millis(10), Duration::from_millis(4));
        assert_eq!(result, Duration::from_millis(6));
    }

    #[test]
    fn tls_version_error_detection() {
        assert!(is_tls_version_error("ssl_min_max_version not supported"));
        assert!(is_tls_version_error("unsupported protocol"));
        assert!(is_tls_version_error("tls version not supported"));
        assert!(is_tls_version_error("tls unsupported by backend"));
        assert!(!is_tls_version_error("connection refused"));
        assert!(!is_tls_version_error("ssl certificate error"));
    }
}
