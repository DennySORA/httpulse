use super::super::{MetricKind, MetricStats, WindowedAggregate};
use super::stats::{compute_stats, is_timeout_error, sample_metric};
use crate::common::time::{Clock, SystemClock};
use crate::config::{ProfileId, SamplingConfig, TargetId, WindowSpec};
use crate::probe::{ProbeResult, ProbeSample};
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
        self.windowed_aggregate_with_clock(key, window, sampling, link_capacity_mbps, &SystemClock)
    }

    pub fn windowed_aggregate_with_clock(
        &self,
        key: ProfileKey,
        window: WindowSpec,
        sampling: &SamplingConfig,
        link_capacity_mbps: Option<f64>,
        clock: &dyn Clock,
    ) -> WindowedAggregate {
        let now = clock.now();
        let cutoff = now
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
                        *error_breakdown.entry(err.kind).or_insert(0) += 1;
                    }
                }
            }
        }

        if let Some(total_values) = metric_values.get(&MetricKind::Total)
            && total_values.len() > 1
        {
            let jitter_values: Vec<f64> = total_values
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .collect();
            metric_values.insert(MetricKind::Jitter, jitter_values);
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
        self.timeseries_with_clock(key, window, metric, link_capacity_mbps, &SystemClock)
    }

    pub fn timeseries_with_clock(
        &self,
        key: ProfileKey,
        window: WindowSpec,
        metric: MetricKind,
        link_capacity_mbps: Option<f64>,
        clock: &dyn Clock,
    ) -> Vec<(f64, f64)> {
        let now = clock.now();
        let window_seconds = window.duration().as_secs_f64();
        let cutoff = now
            .checked_sub(window.duration())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut points = Vec::new();

        if let Some(samples) = self.samples.get(&key) {
            for sample in samples.iter().filter(|s| s.ts >= cutoff) {
                if let ProbeResult::Ok = sample.result
                    && let Some(value) = sample_metric(sample, metric, link_capacity_mbps)
                    && let Ok(age) = now.duration_since(sample.ts)
                {
                    let x = (window_seconds - age.as_secs_f64()).max(0.0);
                    points.push((x, value));
                }
            }
        }

        points
    }

    pub fn timeout_events(&self, key: ProfileKey, window: WindowSpec) -> Vec<f64> {
        self.timeout_events_with_clock(key, window, &SystemClock)
    }

    pub fn timeout_events_with_clock(
        &self,
        key: ProfileKey,
        window: WindowSpec,
        clock: &dyn Clock,
    ) -> Vec<f64> {
        let now = clock.now();
        let window_seconds = window.duration().as_secs_f64();
        let cutoff = now
            .checked_sub(window.duration())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut points = Vec::new();

        if let Some(samples) = self.samples.get(&key) {
            for sample in samples.iter().filter(|s| s.ts >= cutoff) {
                if let ProbeResult::Err(err) = &sample.result
                    && is_timeout_error(&err.kind)
                    && let Ok(age) = now.duration_since(sample.ts)
                {
                    let x = (window_seconds - age.as_secs_f64()).max(0.0);
                    points.push(x);
                }
            }
        }

        points
    }
}

#[cfg(test)]
mod tests;
