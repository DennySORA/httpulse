use super::helpers::{
    fetch_negotiated_protocol, fetch_tcp_info, is_dns_timeout_message, map_curl_error,
    parse_socket_addr, saturating_sub,
};
use crate::config::{ConnReusePolicy, HttpVersion, ProfileConfig, TargetConfig, TlsVersion};
use crate::probe::{
    EbpfConnStatsDelta, ProbeError, ProbeErrorKind, ProbeResult, ProbeSample,
};
use curl::Error as CurlError;
use curl::easy::{
    Easy2, Handler, HttpVersion as CurlHttpVersion, IpResolve, List, SslVersion, WriteError,
};
use std::net::IpAddr;
use std::time::SystemTime;

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

        let configured_tls = match profile.tls {
            TlsVersion::Tls12 => "TLSv1.2",
            TlsVersion::Tls13 => "TLSv1.3",
        };
        let negotiated = fetch_negotiated_protocol(self.easy.raw(), configured_tls);

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

#[cfg(test)]
mod tests;
