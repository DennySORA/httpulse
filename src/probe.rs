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
