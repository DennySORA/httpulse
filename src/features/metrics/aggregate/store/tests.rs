use super::{MetricsStore, ProfileKey};
use crate::common::time::Clock;
use crate::config::{SamplingConfig, WindowSpec};
use crate::metrics::MetricKind;
use crate::probe::{NegotiatedProtocol, ProbeError, ProbeErrorKind, ProbeResult, ProbeSample};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

fn ok_sample(target_id: Uuid, profile_id: Uuid, total_ms: u64) -> ProbeSample {
    ok_sample_at(SystemTime::now(), target_id, profile_id, total_ms)
}

fn ok_sample_at(ts: SystemTime, target_id: Uuid, profile_id: Uuid, total_ms: u64) -> ProbeSample {
    let total = Duration::from_millis(total_ms);
    ProbeSample {
        ts,
        target_id,
        profile_id,
        result: ProbeResult::Ok,
        http_status: Some(200),
        negotiated: NegotiatedProtocol {
            alpn: Some("h2".to_string()),
            tls_version: Some("TLSv1.3".to_string()),
            cipher: None,
        },
        t_dns: Some(Duration::from_millis(2)),
        t_connect: Duration::from_millis(5),
        t_tls: Some(Duration::from_millis(8)),
        t_ttfb: Duration::from_millis(12),
        t_download: Duration::from_millis(total_ms.saturating_sub(12)),
        t_total: total,
        downloaded_bytes: 1024,
        local: None,
        remote: None,
        tcp_info: None,
        ebpf: None,
    }
}

fn error_sample(kind: ProbeErrorKind) -> ProbeSample {
    ProbeSample {
        ts: SystemTime::now(),
        target_id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        result: ProbeResult::Err(ProbeError {
            kind,
            message: "error".to_string(),
        }),
        http_status: None,
        negotiated: NegotiatedProtocol {
            alpn: None,
            tls_version: None,
            cipher: None,
        },
        t_dns: None,
        t_connect: Duration::from_millis(0),
        t_tls: None,
        t_ttfb: Duration::from_millis(0),
        t_download: Duration::from_millis(0),
        t_total: Duration::from_millis(0),
        downloaded_bytes: 0,
        local: None,
        remote: None,
        tcp_info: None,
        ebpf: None,
    }
}

struct FixedClock(SystemTime);

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

#[test]
fn timeout_events_only_include_timeouts() {
    let mut store = MetricsStore::new();
    let target_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    let key = ProfileKey {
        target_id,
        profile_id,
    };

    let mut timeout_sample = error_sample(ProbeErrorKind::HttpTimeout);
    timeout_sample.target_id = target_id;
    timeout_sample.profile_id = profile_id;
    store.push_sample(key, timeout_sample, 16);

    let mut non_timeout = error_sample(ProbeErrorKind::HttpStatusError);
    non_timeout.target_id = target_id;
    non_timeout.profile_id = profile_id;
    store.push_sample(key, non_timeout, 16);

    let events = store.timeout_events(key, WindowSpec::M1);
    assert_eq!(events.len(), 1);
}

#[test]
fn windowed_aggregate_tracks_error_rate_and_totals() {
    let mut store = MetricsStore::new();
    let target_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    let key = ProfileKey {
        target_id,
        profile_id,
    };

    store.push_sample(key, ok_sample(target_id, profile_id, 120), 16);
    let mut timeout_sample = error_sample(ProbeErrorKind::HttpTimeout);
    timeout_sample.target_id = target_id;
    timeout_sample.profile_id = profile_id;
    store.push_sample(key, timeout_sample, 16);
    store.push_sample(key, ok_sample(target_id, profile_id, 240), 16);

    let aggregate = store.windowed_aggregate(key, WindowSpec::M1, &SamplingConfig::default(), None);

    let loss_rate = aggregate
        .by_metric
        .get(&MetricKind::ProbeLossRate)
        .expect("loss rate stats");
    assert_eq!(loss_rate.n, 3);
    let expected = 1.0 / 3.0;
    let actual = loss_rate.mean.expect("mean");
    assert!((actual - expected).abs() < 1e-6);

    let total_stats = aggregate
        .by_metric
        .get(&MetricKind::Total)
        .expect("total stats");
    assert_eq!(total_stats.n, 2);
    assert_eq!(
        *aggregate
            .error_breakdown
            .get(&ProbeErrorKind::HttpTimeout)
            .expect("timeout count"),
        1
    );
}

#[test]
fn push_sample_respects_max_points() {
    let mut store = MetricsStore::new();
    let target_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    let key = ProfileKey {
        target_id,
        profile_id,
    };

    store.push_sample(key, ok_sample(target_id, profile_id, 10), 2);
    store.push_sample(key, ok_sample(target_id, profile_id, 20), 2);
    store.push_sample(key, ok_sample(target_id, profile_id, 30), 2);

    let aggregate = store.windowed_aggregate(key, WindowSpec::M1, &SamplingConfig::default(), None);
    let total_stats = aggregate
        .by_metric
        .get(&MetricKind::Total)
        .expect("total stats");
    assert_eq!(total_stats.n, 2);
    assert_eq!(total_stats.min, Some(20.0));
    assert_eq!(total_stats.max, Some(30.0));
    assert_eq!(total_stats.last, Some(30.0));
}

#[test]
fn timeseries_with_clock_uses_fixed_now() {
    let mut store = MetricsStore::new();
    let target_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    let key = ProfileKey {
        target_id,
        profile_id,
    };

    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let sample_ts = now - Duration::from_secs(10);
    store.push_sample(key, ok_sample_at(sample_ts, target_id, profile_id, 120), 16);

    let points = store.timeseries_with_clock(
        key,
        WindowSpec::M1,
        MetricKind::Total,
        None,
        &FixedClock(now),
    );
    assert_eq!(points.len(), 1);
    let (x, _) = points[0];
    assert!((x - 50.0).abs() < 1e-6);
}
