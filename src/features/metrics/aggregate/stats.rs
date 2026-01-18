use crate::config::SamplingConfig;
use crate::probe::{ProbeErrorKind, ProbeSample};
use hdrhistogram::Histogram;

use super::super::{MetricKind, MetricStats};

pub(super) fn is_timeout_error(kind: &ProbeErrorKind) -> bool {
    kind.is_timeout()
}

pub(super) fn compute_stats(
    values: &[f64],
    sampling: &SamplingConfig,
    use_histogram: bool,
) -> MetricStats {
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

pub(super) fn sample_metric(
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
