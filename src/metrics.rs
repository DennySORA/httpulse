use crate::config::{ProfileId, TargetId, WindowSpec};
use crate::probe::ProbeErrorKind;
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MetricKind {
    Dns,
    Connect,
    Tls,
    Ttfb,
    Download,
    Total,
    Rtt,
    RttVar,
    Jitter,
    Retrans,
    Reordering,
    DupAcks,
    ProbeLossRate,
    TransportLoss,
    GoodputBps,
    BandwidthUtilization,
    Cwnd,
    Ssthresh,
}

impl MetricKind {
    pub fn unit(&self) -> &'static str {
        match self {
            MetricKind::Dns
            | MetricKind::Connect
            | MetricKind::Tls
            | MetricKind::Ttfb
            | MetricKind::Download
            | MetricKind::Total
            | MetricKind::Rtt
            | MetricKind::RttVar
            | MetricKind::Jitter => "ms",
            MetricKind::GoodputBps => "Mbps",
            MetricKind::BandwidthUtilization | MetricKind::ProbeLossRate => "%",
            _ => "",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MetricStats {
    pub n: u64,
    pub last: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct WindowedAggregate {
    pub window: WindowSpec,
    pub by_metric: HashMap<MetricKind, MetricStats>,
    pub error_breakdown: HashMap<ProbeErrorKind, u64>,
}

#[derive(Clone, Debug)]
pub struct ProfileAggregate {
    pub target_id: TargetId,
    pub profile_id: ProfileId,
    pub windows: Vec<WindowedAggregate>,
}

#[derive(Clone, Debug)]
pub struct CandidateDomain {
    pub domain: String,
    pub ip: Option<IpAddr>,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TargetViewModel {
    pub target_id: TargetId,
    pub url: String,
    pub profiles: Vec<ProfileAggregate>,
    pub candidates_hint: Vec<CandidateDomain>,
}
