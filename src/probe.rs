use crate::config::{ProfileId, TargetId};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct ProbeSample {
    pub ts: SystemTime,
    pub target_id: TargetId,
    pub profile_id: ProfileId,
    pub result: ProbeResult,
    pub http_status: Option<u16>,
    pub negotiated: NegotiatedProtocol,
    pub t_dns: Option<Duration>,
    pub t_connect: Duration,
    pub t_tls: Option<Duration>,
    pub t_ttfb: Duration,
    pub t_download: Duration,
    pub t_total: Duration,
    pub downloaded_bytes: u64,
    pub local: Option<SocketAddr>,
    pub remote: Option<SocketAddr>,
    pub tcp_info: Option<TcpInfoSnapshot>,
    pub ebpf: Option<EbpfConnStatsDelta>,
}

#[derive(Clone, Debug)]
pub enum ProbeResult {
    Ok,
    Err(ProbeError),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ProbeErrorKind {
    DnsTimeout,
    DnsNxDomain,
    DnsServFail,
    DnsOther,
    ConnectTimeout,
    ConnectRefused,
    ConnectNoRoute,
    ConnectOther,
    TlsHandshakeFailed,
    TlsVersionMismatch,
    AlpnFailed,
    HttpTimeout,
    HttpProtocolError,
    HttpStatusError,
    ReadTimeout,
    IoError,
}

impl ProbeErrorKind {
    pub fn label(&self) -> &'static str {
        match self {
            ProbeErrorKind::DnsTimeout => "dns_timeout",
            ProbeErrorKind::DnsNxDomain => "dns_nxdomain",
            ProbeErrorKind::DnsServFail => "dns_servfail",
            ProbeErrorKind::DnsOther => "dns_other",
            ProbeErrorKind::ConnectTimeout => "connect_timeout",
            ProbeErrorKind::ConnectRefused => "connect_refused",
            ProbeErrorKind::ConnectNoRoute => "connect_no_route",
            ProbeErrorKind::ConnectOther => "connect_other",
            ProbeErrorKind::TlsHandshakeFailed => "tls_handshake_failed",
            ProbeErrorKind::TlsVersionMismatch => "tls_version_mismatch",
            ProbeErrorKind::AlpnFailed => "alpn_failed",
            ProbeErrorKind::HttpTimeout => "http_timeout",
            ProbeErrorKind::HttpProtocolError => "http_protocol_error",
            ProbeErrorKind::HttpStatusError => "http_status_error",
            ProbeErrorKind::ReadTimeout => "read_timeout",
            ProbeErrorKind::IoError => "io_error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProbeError {
    pub kind: ProbeErrorKind,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct NegotiatedProtocol {
    pub alpn: Option<String>,
    pub tls_version: Option<String>,
    pub cipher: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TcpInfoSnapshot {
    pub rtt_us: Option<u32>,
    pub rttvar_us: Option<u32>,
    pub total_retrans: Option<u32>,
    pub lost: Option<u32>,
    pub reordering: Option<u32>,
    pub snd_cwnd: Option<u32>,
    pub snd_ssthresh: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct EbpfConnStatsDelta {
    pub retrans: u32,
    pub dup_acks: u32,
    pub conn_events: u32,
}
