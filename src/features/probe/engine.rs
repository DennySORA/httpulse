use crate::config::{ConnReusePolicy, HttpVersion, ProfileConfig, TargetConfig, TlsVersion};
use crate::probe::{
    EbpfConnStatsDelta, NegotiatedProtocol, ProbeError, ProbeErrorKind, ProbeResult, ProbeSample,
    TcpInfoSnapshot,
};
use curl::Error as CurlError;
use curl::easy::{
    Easy2, Handler, HttpVersion as CurlHttpVersion, IpResolve, List, SslVersion, WriteError,
};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct BodyCollector {
    bytes: u64,
    limit: u64,
    limit_reached: bool,
}

impl BodyCollector {
    fn reset(&mut self, limit: u64) {
        self.bytes = 0;
        self.limit = limit;
        self.limit_reached = false;
    }
}

impl Handler for BodyCollector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        let len = data.len() as u64;
        let take = if self.limit == 0 {
            len
        } else {
            let remaining = self.limit.saturating_sub(self.bytes);
            len.min(remaining)
        };

        self.bytes = self.bytes.saturating_add(take);
        if self.limit > 0 && self.bytes >= self.limit {
            self.limit_reached = true;
        }

        Ok(data.len())
    }

    fn progress(&mut self, _dltotal: f64, dlnow: f64, _ultotal: f64, _ulnow: f64) -> bool {
        if self.limit == 0 {
            return true;
        }

        if self.limit_reached {
            return false;
        }

        if dlnow >= self.limit as f64 {
            self.limit_reached = true;
            return false;
        }

        true
    }
}

pub struct ProbeClient {
    easy: Easy2<BodyCollector>,
}

impl ProbeClient {
    pub fn new() -> Result<Self, CurlError> {
        let mut easy = Easy2::new(BodyCollector::default());
        easy.follow_location(false)?;
        easy.accept_encoding("")?;
        Ok(Self { easy })
    }

    pub fn probe(
        &mut self,
        target: &TargetConfig,
        profile: &ProfileConfig,
        resolved_ip: Option<IpAddr>,
    ) -> ProbeSample {
        let host_is_ip = target
            .url
            .host_str()
            .and_then(|host| host.parse::<IpAddr>().ok())
            .is_some();
        let retry_on_dns_timeout = target.dns_enabled && !host_is_ip;

        let ip_modes = [IpResolve::Any, IpResolve::V4];
        let ip_modes = if retry_on_dns_timeout {
            &ip_modes[..]
        } else {
            &ip_modes[..1]
        };

        let mut last_sample = None;
        for (index, ip_mode) in ip_modes.iter().enumerate() {
            let (sample, dns_timeout) = self.probe_once(target, profile, resolved_ip, *ip_mode);
            let should_retry = dns_timeout && index + 1 < ip_modes.len();
            if should_retry {
                last_sample = Some(sample);
                continue;
            }
            return sample;
        }

        last_sample.expect("probe attempts should return a sample")
    }

    fn probe_once(
        &mut self,
        target: &TargetConfig,
        profile: &ProfileConfig,
        resolved_ip: Option<IpAddr>,
        ip_resolve: IpResolve,
    ) -> (ProbeSample, bool) {
        let start_ts = SystemTime::now();
        let read_limit = if profile.method == crate::config::ProbeMethod::Head {
            0
        } else {
            profile.max_read_bytes as u64
        };
        self.easy.reset();
        self.easy.get_mut().reset(read_limit);
        let _ = self.easy.follow_location(false);
        let _ = self.easy.accept_encoding("");
        let _ = self.easy.progress(true);
        let _ = self.easy.ip_resolve(ip_resolve);

        let url = target.url.as_str();
        let _ = self.easy.url(url);
        let _ = self.easy.timeout(target.timeout_total);

        if let Some(breakdown) = target.timeout_breakdown {
            let _ = self.easy.connect_timeout(breakdown.connect);
        }

        let _ = self.easy.http_version(match profile.http {
            HttpVersion::H1 => CurlHttpVersion::V11,
            HttpVersion::H2 => CurlHttpVersion::V2TLS,
        });

        let tls_version = match profile.tls {
            TlsVersion::Tls12 => SslVersion::Tlsv12,
            TlsVersion::Tls13 => SslVersion::Tlsv13,
        };
        let _ = self.easy.ssl_min_max_version(tls_version, tls_version);

        match profile.conn_reuse {
            ConnReusePolicy::Warm => {
                let _ = self.easy.fresh_connect(false);
                let _ = self.easy.forbid_reuse(false);
            }
            ConnReusePolicy::Cold => {
                let _ = self.easy.fresh_connect(true);
                let _ = self.easy.forbid_reuse(true);
            }
        }

        if profile.method == crate::config::ProbeMethod::Head {
            let _ = self.easy.nobody(true);
        } else {
            let _ = self.easy.nobody(false);
        }

        if !profile.headers.is_empty() {
            let mut list = List::new();
            for (name, value) in &profile.headers {
                let header = format!("{name}: {}", value.expose());
                let _ = list.append(&header);
            }
            let _ = self.easy.http_headers(list);
        }

        if !target.dns_enabled
            && let Some(ip) = resolved_ip
            && let Some(host) = target.url.host_str()
        {
            let port = target.url.port_or_known_default().unwrap_or_else(|| {
                if target.url.scheme() == "https" {
                    443
                } else {
                    80
                }
            });
            let mut list = List::new();
            let entry = format!("{host}:{port}:{ip}");
            let _ = list.append(&entry);
            let _ = self.easy.resolve(list);
        }

        let mut probe_result = ProbeResult::Ok;
        let perform_result = self.easy.perform();
        let was_aborted_by_limit = self.easy.get_ref().limit_reached;
        let mut dns_timeout = false;

        if let Err(err) = &perform_result
            && err.is_operation_timedout()
        {
            dns_timeout = is_dns_timeout_message(&err.to_string());
        }

        let http_status = self.easy.response_code().ok().map(|code| code as u16);
        if let Some(status) = http_status
            && status >= 400
        {
            probe_result = ProbeResult::Err(ProbeError {
                kind: ProbeErrorKind::HttpStatusError,
                message: format!("HTTP status {status}"),
            });
        }

        let mut aborted_by_limit = false;
        if let Err(err) = perform_result {
            aborted_by_limit =
                was_aborted_by_limit && (err.is_write_error() || err.is_aborted_by_callback());
            if !aborted_by_limit {
                probe_result = ProbeResult::Err(map_curl_error(&err));
            }
        }
        let dns_timeout = dns_timeout && !aborted_by_limit;

        let t_total = self.easy.total_time().unwrap_or_default();
        let t_dns_raw = self.easy.namelookup_time().unwrap_or_default();
        let t_connect_raw = self.easy.connect_time().unwrap_or(t_dns_raw);
        let t_tls_raw = self.easy.appconnect_time().unwrap_or(t_connect_raw);
        let t_ttfb_raw = self.easy.starttransfer_time().unwrap_or(t_tls_raw);

        let t_connect = saturating_sub(t_connect_raw, t_dns_raw);
        let t_tls = saturating_sub(t_tls_raw, t_connect_raw);
        let t_ttfb = saturating_sub(t_ttfb_raw, t_tls_raw);
        let t_download = saturating_sub(t_total, t_ttfb_raw);

        let downloaded_bytes = self.easy.get_ref().bytes;

        // let negotiated = fetch_negotiated_protocol(self.easy.raw());
        let negotiated = NegotiatedProtocol {
            alpn: Some(match profile.http {
                HttpVersion::H1 => "http/1.1".to_string(),
                HttpVersion::H2 => "h2".to_string(),
            }),
            tls_version: Some(match profile.tls {
                TlsVersion::Tls12 => "TLSv1.2".to_string(),
                TlsVersion::Tls13 => "TLSv1.3".to_string(),
            }),
            cipher: None,
        };

        let local = parse_socket_addr(
            self.easy.local_ip().ok().flatten(),
            self.easy.local_port().ok(),
        );
        let remote = parse_socket_addr(
            self.easy.primary_ip().ok().flatten(),
            self.easy.primary_port().ok(),
        );

        let tcp_info = fetch_tcp_info(self.easy.raw());

        let sample = ProbeSample {
            ts: start_ts,
            target_id: target.id,
            profile_id: profile.id,
            result: probe_result,
            http_status,
            negotiated,
            t_dns: if target.dns_enabled {
                Some(t_dns_raw)
            } else {
                None
            },
            t_connect,
            t_tls: Some(t_tls),
            t_ttfb,
            t_download,
            t_total,
            downloaded_bytes,
            local,
            remote,
            tcp_info,
            ebpf: None::<EbpfConnStatsDelta>,
        };

        (sample, dns_timeout)
    }
}

fn map_curl_error(err: &CurlError) -> ProbeError {
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
        ProbeErrorKind::TlsHandshakeFailed
    } else if err.is_http_returned_error() {
        ProbeErrorKind::HttpStatusError
    } else if err.is_read_error() {
        ProbeErrorKind::ReadTimeout
    } else {
        ProbeErrorKind::IoError
    };

    ProbeError {
        kind,
        message: err.to_string(),
    }
}

fn is_dns_timeout_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("resolving timed out")
}

// fn fetch_negotiated_protocol(handle: *mut curl_sys::CURL) -> NegotiatedProtocol {
//     let alpn = unsafe {
//         let mut version_long = 0;
//         curl_sys::curl_easy_getinfo(handle, curl_sys::CURLINFO_HTTP_VERSION, &mut version_long);
//         match version_long as i32 {
//             curl_sys::CURL_HTTP_VERSION_1_0 => Some("http/1.0".to_string()),
//             curl_sys::CURL_HTTP_VERSION_1_1 => Some("http/1.1".to_string()),
//             curl_sys::CURL_HTTP_VERSION_2 => Some("h2".to_string()),
//             curl_sys::CURL_HTTP_VERSION_3 => Some("h3".to_string()),
//             _ => None,
//         }
//     };
//
//     let tls_version = unsafe {
//         let mut ptr: *const libc::c_char = std::ptr::null();
//         if curl_sys::curl_easy_getinfo(handle, curl_sys::CURLINFO_TLS_VERSION, &mut ptr)
//             == curl_sys::CURLE_OK
//             && !ptr.is_null()
//         {
//             Some(
//                 std::ffi::CStr::from_ptr(ptr)
//                     .to_string_lossy()
//                     .into_owned(),
//             )
//         } else {
//             None
//         }
//     };
//
//     let cipher = unsafe {
//         let mut ptr: *const libc::c_char = std::ptr::null();
//         if curl_sys::curl_easy_getinfo(handle, curl_sys::CURLINFO_SSL_CIPHER, &mut ptr)
//             == curl_sys::CURLE_OK
//             && !ptr.is_null()
//         {
//             Some(
//                 std::ffi::CStr::from_ptr(ptr)
//                     .to_string_lossy()
//                     .into_owned(),
//             )
//         } else {
//             None
//         }
//     };
//
//     NegotiatedProtocol {
//         alpn,
//         tls_version,
//         cipher,
//     }
// }

fn parse_socket_addr(ip: Option<&str>, port: Option<u16>) -> Option<SocketAddr> {
    let port = port?;
    let ip = ip?.parse::<IpAddr>().ok()?;
    Some(SocketAddr::new(ip, port))
}

fn saturating_sub(left: Duration, right: Duration) -> Duration {
    left.checked_sub(right).unwrap_or(Duration::from_millis(0))
}

fn fetch_tcp_info(handle: *mut curl_sys::CURL) -> Option<TcpInfoSnapshot> {
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

#[cfg(test)]
mod tests {
    use super::BodyCollector;
    use curl::easy::Handler;
    use std::time::Duration;

    #[test]
    fn body_collector_no_limit_counts_bytes() {
        let mut collector = BodyCollector::default();
        collector.reset(0);
        let data = vec![0u8; 8];
        let wrote = collector.write(&data).expect("write");
        assert_eq!(wrote, data.len());
        assert_eq!(collector.bytes, 8);
        assert!(!collector.limit_reached);
    }

    #[test]
    fn body_collector_caps_bytes_when_limit_hit() {
        let mut collector = BodyCollector::default();
        collector.reset(5);
        let data = vec![0u8; 10];
        let wrote = collector.write(&data).expect("write");
        assert_eq!(wrote, data.len());
        assert_eq!(collector.bytes, 5);
        assert!(collector.limit_reached);
    }

    #[test]
    fn body_collector_caps_bytes_after_partial() {
        let mut collector = BodyCollector::default();
        collector.reset(5);
        let first = vec![0u8; 3];
        let wrote_first = collector.write(&first).expect("write");
        assert_eq!(wrote_first, 3);
        assert_eq!(collector.bytes, 3);
        assert!(!collector.limit_reached);

        let second = vec![0u8; 4];
        let wrote_second = collector.write(&second).expect("write");
        assert_eq!(wrote_second, second.len());
        assert_eq!(collector.bytes, 5);
        assert!(collector.limit_reached);
    }

    #[test]
    fn body_collector_progress_aborts_after_limit() {
        let mut collector = BodyCollector::default();
        collector.reset(5);
        let data = vec![0u8; 5];
        let _ = collector.write(&data).expect("write");
        assert!(collector.limit_reached);
        assert!(!collector.progress(0.0, 5.0, 0.0, 0.0));
    }

    #[test]
    fn body_collector_progress_allows_below_limit() {
        let mut collector = BodyCollector::default();
        collector.reset(5);
        assert!(collector.progress(0.0, 2.0, 0.0, 0.0));
    }

    #[test]
    fn dns_timeout_message_detection() {
        assert!(super::is_dns_timeout_message(
            "[28] Timeout was reached (Resolving timed out after 10000 milliseconds)"
        ));
        assert!(!super::is_dns_timeout_message(
            "[28] Timeout was reached (Operation timed out after 10000 milliseconds)"
        ));
    }

    #[test]
    fn parse_socket_addr_accepts_valid_values() {
        let addr = super::parse_socket_addr(Some("127.0.0.1"), Some(443)).expect("addr");
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 443);
    }

    #[test]
    fn parse_socket_addr_rejects_invalid_values() {
        assert!(super::parse_socket_addr(Some("not-an-ip"), Some(80)).is_none());
        assert!(super::parse_socket_addr(None, Some(80)).is_none());
        assert!(super::parse_socket_addr(Some("127.0.0.1"), None).is_none());
    }

    #[test]
    fn saturating_sub_handles_underflow() {
        let result = super::saturating_sub(Duration::from_millis(5), Duration::from_millis(10));
        assert_eq!(result, Duration::from_millis(0));
    }

    #[test]
    fn saturating_sub_returns_difference() {
        let result = super::saturating_sub(Duration::from_millis(10), Duration::from_millis(4));
        assert_eq!(result, Duration::from_millis(6));
    }
}
