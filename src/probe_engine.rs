use crate::config::{ConnReusePolicy, HttpVersion, ProfileConfig, TargetConfig, TlsVersion};
use crate::probe::{
    EbpfConnStatsDelta, NegotiatedProtocol, ProbeError, ProbeErrorKind, ProbeResult, ProbeSample,
    TcpInfoSnapshot,
};
use curl::Error as CurlError;
use curl::easy::{Easy2, Handler, HttpVersion as CurlHttpVersion, List, SslVersion, WriteError};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct BodyCollector {
    bytes: u64,
}

impl BodyCollector {
    fn reset(&mut self) {
        self.bytes = 0;
    }
}

impl Handler for BodyCollector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.bytes = self.bytes.saturating_add(data.len() as u64);
        Ok(data.len())
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
        let start_ts = SystemTime::now();
        self.easy.get_mut().reset();
        self.easy.reset();

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
            if profile.max_read_bytes > 0 {
                let range = format!("0-{}", profile.max_read_bytes.saturating_sub(1));
                let _ = self.easy.range(&range);
            }
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

        let http_status = self.easy.response_code().ok().map(|code| code as u16);
        if let Some(status) = http_status
            && status >= 400
        {
            probe_result = ProbeResult::Err(ProbeError {
                kind: ProbeErrorKind::HttpStatusError,
                message: format!("HTTP status {status}"),
            });
        }

        if let Err(err) = perform_result {
            probe_result = ProbeResult::Err(map_curl_error(&err));
        }

        let t_total = self.easy.total_time().unwrap_or_default();
        let t_dns_raw = self.easy.namelookup_time().unwrap_or_default();
        let t_connect_raw = self.easy.connect_time().unwrap_or(t_dns_raw);
        let t_tls_raw = self.easy.appconnect_time().unwrap_or(t_connect_raw);
        let t_ttfb_raw = self.easy.starttransfer_time().unwrap_or(t_tls_raw);

        let t_connect = saturating_sub(t_connect_raw, t_dns_raw);
        let t_tls = saturating_sub(t_tls_raw, t_connect_raw);
        let t_ttfb = saturating_sub(t_ttfb_raw, t_tls_raw);
        let t_download = saturating_sub(t_total, t_ttfb_raw);

        let downloaded_bytes = self
            .easy
            .download_size()
            .map(|v| v.max(0.0) as u64)
            .unwrap_or(self.easy.get_ref().bytes);

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

        ProbeSample {
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
        }
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
