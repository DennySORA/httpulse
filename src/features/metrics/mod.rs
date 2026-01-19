pub mod aggregate;

use crate::config::{ProfileId, TargetId, WindowSpec};
use crate::probe::ProbeErrorKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    pub fn iter_all() -> &'static [MetricKind] {
        &[
            MetricKind::Dns,
            MetricKind::Connect,
            MetricKind::Tls,
            MetricKind::Ttfb,
            MetricKind::Download,
            MetricKind::Total,
            MetricKind::Rtt,
            MetricKind::RttVar,
            MetricKind::Jitter,
            MetricKind::Retrans,
            MetricKind::Reordering,
            MetricKind::DupAcks,
            MetricKind::ProbeLossRate,
            MetricKind::TransportLoss,
            MetricKind::GoodputBps,
            MetricKind::BandwidthUtilization,
            MetricKind::Cwnd,
            MetricKind::Ssthresh,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            MetricKind::Dns => "dns",
            MetricKind::Connect => "connect",
            MetricKind::Tls => "tls",
            MetricKind::Ttfb => "ttfb",
            MetricKind::Download => "download",
            MetricKind::Total => "total",
            MetricKind::Rtt => "rtt",
            MetricKind::RttVar => "rttvar",
            MetricKind::Jitter => "jitter",
            MetricKind::Retrans => "retrans",
            MetricKind::Reordering => "reorder",
            MetricKind::DupAcks => "dupack",
            MetricKind::ProbeLossRate => "probe_loss",
            MetricKind::TransportLoss => "transport_loss",
            MetricKind::GoodputBps => "goodput_bps",
            MetricKind::BandwidthUtilization => "utilization",
            MetricKind::Cwnd => "cwnd",
            MetricKind::Ssthresh => "ssthresh",
        }
    }

    pub fn is_latency_metric(self) -> bool {
        matches!(
            self,
            MetricKind::Dns
                | MetricKind::Connect
                | MetricKind::Tls
                | MetricKind::Ttfb
                | MetricKind::Download
                | MetricKind::Total
                | MetricKind::Rtt
                | MetricKind::RttVar
                | MetricKind::Jitter
        )
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

impl MetricStats {
    pub fn empty() -> Self {
        Self {
            n: 0,
            last: None,
            min: None,
            max: None,
            mean: None,
            stddev: None,
            p50: None,
            p90: None,
            p99: None,
        }
    }

    pub fn from_scalar(value: Option<f64>, n: u64) -> Self {
        Self {
            n,
            last: value,
            min: value,
            max: value,
            mean: value,
            stddev: value.map(|_| 0.0),
            p50: value,
            p90: value,
            p99: value,
        }
    }
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
