use crate::config::{ProfileId, SamplingConfig, TargetId, WindowSpec};
use crate::metrics::{MetricKind, MetricStats, WindowedAggregate};
use crate::probe::{ProbeResult, ProbeSample};
use hdrhistogram::Histogram;
use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ProfileKey {
    pub target_id: TargetId,
    pub profile_id: ProfileId,
}

#[derive(Default)]
pub struct MetricsStore {
    samples: HashMap<ProfileKey, VecDeque<ProbeSample>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    pub fn push_sample(&mut self, key: ProfileKey, sample: ProbeSample, max_points: usize) {
        let queue = self.samples.entry(key).or_default();
        queue.push_back(sample);
        while queue.len() > max_points {
            queue.pop_front();
        }
    }

    pub fn windowed_aggregate(
        &self,
        key: ProfileKey,
        window: WindowSpec,
        sampling: &SamplingConfig,
        link_capacity_mbps: Option<f64>,
    ) -> WindowedAggregate {
        let cutoff = SystemTime::now()
            .checked_sub(window.duration())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut error_breakdown = HashMap::new();
        let mut total_samples = 0u64;
        let mut error_samples = 0u64;
        let mut metric_values: HashMap<MetricKind, Vec<f64>> = HashMap::new();

        if let Some(samples) = self.samples.get(&key) {
            for sample in samples.iter().filter(|s| s.ts >= cutoff) {
                total_samples += 1;
                match &sample.result {
                    ProbeResult::Ok => {
                        for &metric in MetricKind::iter_all() {
                            if metric == MetricKind::ProbeLossRate {
                                continue;
                            }
                            if let Some(value) = sample_metric(sample, metric, link_capacity_mbps) {
                                metric_values.entry(metric).or_default().push(value);
                            }
                        }
                    }
                    ProbeResult::Err(err) => {
                        error_samples += 1;
                        *error_breakdown.entry(err.kind.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        if let Some(total_values) = metric_values.get(&MetricKind::Total) {
            if total_values.len() > 1 {
                let jitter_values: Vec<f64> =
                    total_values.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
                metric_values.insert(MetricKind::Jitter, jitter_values);
            }
        }

        let mut by_metric = HashMap::new();
        for &metric in MetricKind::iter_all() {
            if metric == MetricKind::ProbeLossRate {
                let rate = if total_samples == 0 {
                    None
                } else {
                    Some(error_samples as f64 / total_samples as f64)
                };
                by_metric.insert(metric, MetricStats::from_scalar(rate, total_samples));
                continue;
            }

            let values = metric_values.remove(&metric).unwrap_or_default();
            by_metric.insert(
                metric,
                compute_stats(&values, sampling, metric.is_latency_metric()),
            );
        }

        WindowedAggregate {
            window,
            by_metric,
            error_breakdown,
        }
    }

    pub fn timeseries(
        &self,
        key: ProfileKey,
        window: WindowSpec,
        metric: MetricKind,
        link_capacity_mbps: Option<f64>,
    ) -> Vec<(f64, f64)> {
        let window_seconds = window.duration().as_secs_f64();
        let cutoff = SystemTime::now()
            .checked_sub(window.duration())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut points = Vec::new();

        if let Some(samples) = self.samples.get(&key) {
            for sample in samples.iter().filter(|s| s.ts >= cutoff) {
                if let ProbeResult::Ok = sample.result {
                    if let Some(value) = sample_metric(sample, metric, link_capacity_mbps) {
                        if let Ok(age) = SystemTime::now().duration_since(sample.ts) {
                            let x = (window_seconds - age.as_secs_f64()).max(0.0);
                            points.push((x, value));
                        }
                    }
                }
            }
        }

        points
    }
}

fn compute_stats(values: &[f64], sampling: &SamplingConfig, use_histogram: bool) -> MetricStats {
    if values.is_empty() {
        return MetricStats::empty();
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut sum = 0.0;
    for value in values {
        min = min.min(*value);
        max = max.max(*value);
        sum += value;
    }
    let mean = sum / values.len() as f64;
    let mut variance_sum = 0.0;
    for value in values {
        let diff = value - mean;
        variance_sum += diff * diff;
    }
    let stddev = (variance_sum / values.len() as f64).sqrt();

    let (p50, p90, p99) = if use_histogram {
        let low = sampling.histogram.latency_low_ms.max(1) * 1_000;
        let high = sampling.histogram.latency_high_ms.max(1) * 1_000;
        let mut histogram = Histogram::<u64>::new_with_bounds(low, high, sampling.histogram.sigfig)
            .unwrap_or_else(|_| Histogram::<u64>::new(3).unwrap());
        for value in values {
            let micros = (*value * 1000.0).max(0.0) as u64;
            let _ = histogram.record(micros);
        }
        (
            histogram.value_at_quantile(0.50) as f64 / 1000.0,
            histogram.value_at_quantile(0.90) as f64 / 1000.0,
            histogram.value_at_quantile(0.99) as f64 / 1000.0,
        )
    } else {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        (
            quantile(&sorted, 0.50),
            quantile(&sorted, 0.90),
            quantile(&sorted, 0.99),
        )
    };

    MetricStats {
        n: values.len() as u64,
        last: values.last().copied(),
        min: Some(min),
        max: Some(max),
        mean: Some(mean),
        stddev: Some(stddev),
        p50: Some(p50),
        p90: Some(p90),
        p99: Some(p99),
    }
}

fn quantile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = ((values.len() - 1) as f64 * q).round() as usize;
    values[idx]
}

fn sample_metric(
    sample: &ProbeSample,
    metric: MetricKind,
    link_capacity_mbps: Option<f64>,
) -> Option<f64> {
    match metric {
        MetricKind::Dns => sample.t_dns.map(|t| t.as_secs_f64() * 1000.0),
        MetricKind::Connect => Some(sample.t_connect.as_secs_f64() * 1000.0),
        MetricKind::Tls => sample.t_tls.map(|t| t.as_secs_f64() * 1000.0),
        MetricKind::Ttfb => Some(sample.t_ttfb.as_secs_f64() * 1000.0),
        MetricKind::Download => Some(sample.t_download.as_secs_f64() * 1000.0),
        MetricKind::Total => Some(sample.t_total.as_secs_f64() * 1000.0),
        MetricKind::Rtt => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.rtt_us)
            .map(|v| v as f64 / 1000.0),
        MetricKind::RttVar => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.rttvar_us)
            .map(|v| v as f64 / 1000.0),
        MetricKind::Jitter => None,
        MetricKind::Retrans => sample.ebpf.as_ref().map(|v| v.retrans as f64).or_else(|| {
            sample
                .tcp_info
                .as_ref()
                .and_then(|info| info.total_retrans)
                .map(|v| v as f64)
        }),
        MetricKind::Reordering => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.reordering)
            .map(|v| v as f64),
        MetricKind::DupAcks => sample.ebpf.as_ref().map(|v| v.dup_acks as f64),
        MetricKind::TransportLoss => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.lost)
            .map(|v| v as f64),
        MetricKind::GoodputBps => {
            let seconds = sample.t_download.as_secs_f64();
            if seconds > 0.0 {
                Some(sample.downloaded_bytes as f64 * 8.0 / seconds)
            } else {
                None
            }
        }
        MetricKind::BandwidthUtilization => {
            let capacity = link_capacity_mbps?;
            let goodput = sample_metric(sample, MetricKind::GoodputBps, link_capacity_mbps)?;
            let capacity_bps = capacity * 1_000_000.0;
            Some(goodput / capacity_bps)
        }
        MetricKind::Cwnd => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.snd_cwnd)
            .map(|v| v as f64),
        MetricKind::Ssthresh => sample
            .tcp_info
            .as_ref()
            .and_then(|info| info.snd_ssthresh)
            .map(|v| v as f64),
        MetricKind::ProbeLossRate => None,
    }
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

impl MetricKind {
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
