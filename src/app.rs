use crate::config::{default_profiles, GlobalConfig, ProfileConfig, TargetConfig};
use crate::metrics::{MetricKind, WindowedAggregate};
use crate::metrics_aggregate::{MetricsStore, ProfileKey};
use crate::probe::{ProbeErrorKind, ProbeSample};
use crate::runtime::{spawn_profile_worker, ControlMessage, WorkerHandle};
use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileViewMode {
    Single,
    Compare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetPaneMode {
    Split,
    Chart,
    Metrics,
    Summary,
}

impl TargetPaneMode {
    pub fn cycle(self) -> Self {
        match self {
            TargetPaneMode::Split => TargetPaneMode::Chart,
            TargetPaneMode::Chart => TargetPaneMode::Metrics,
            TargetPaneMode::Metrics => TargetPaneMode::Summary,
            TargetPaneMode::Summary => TargetPaneMode::Split,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TargetPaneMode::Split => "Split",
            TargetPaneMode::Chart => "Chart",
            TargetPaneMode::Metrics => "Metrics",
            TargetPaneMode::Summary => "Summary",
        }
    }
}

pub struct AppState {
    pub global: GlobalConfig,
    pub metrics: MetricsStore,
    pub targets: Vec<TargetRuntime>,
    pub selected_target: usize,
    pub selected_metric: MetricKind,
    pub selected_metrics: HashSet<MetricKind>,
    pub window: crate::config::WindowSpec,
}

pub struct TargetRuntime {
    pub config: TargetConfig,
    pub paused: bool,
    pub last_ip: Option<IpAddr>,
    pub profiles: Vec<ProfileRuntime>,
    pub view_mode: ProfileViewMode,
    pub selected_profile: usize,
    pub pane_mode: TargetPaneMode,
}

pub struct ProfileRuntime {
    pub config: ProfileConfig,
    pub worker: WorkerHandle,
    pub last_sample: Option<ProbeSample>,
    pub last_error: Option<ProbeErrorKind>,
}

#[derive(Clone, Debug, Default)]
pub struct GlobalSummary {
    pub samples: u64,
    pub requests: u64,
    pub successes: u64,
    pub timeouts: u64,
    pub errors: BTreeMap<ProbeErrorKind, u64>,
}

impl AppState {
    pub fn new(global: GlobalConfig) -> Self {
        let mut selected_metrics = HashSet::new();
        selected_metrics.insert(MetricKind::Total);
        Self {
            global: global.clone(),
            metrics: MetricsStore::new(),
            targets: Vec::new(),
            selected_target: 0,
            selected_metric: MetricKind::Total,
            selected_metrics,
            window: global.default_window,
        }
    }

    pub fn add_target(
        &mut self,
        url: Url,
        profiles: Option<Vec<ProfileConfig>>,
        sample_tx: crossbeam_channel::Sender<ProbeSample>,
    ) {
        let profiles = profiles.unwrap_or_else(default_profiles);
        let mut target = TargetConfig::new(url, profiles.clone());
        target.sampling = crate::config::SamplingConfig::default();
        let mut profile_runtimes = Vec::new();
        for profile in profiles {
            let worker = spawn_profile_worker(target.clone(), profile.clone(), sample_tx.clone());
            profile_runtimes.push(ProfileRuntime {
                config: profile,
                worker,
                last_sample: None,
                last_error: None,
            });
        }

        self.targets.push(TargetRuntime {
            config: target,
            paused: false,
            last_ip: None,
            profiles: profile_runtimes,
            view_mode: ProfileViewMode::Single,
            selected_profile: 0,
            pane_mode: TargetPaneMode::Split,
        });
        self.selected_target = self.targets.len().saturating_sub(1);
    }

    pub fn remove_target(&mut self, index: usize) {
        if index >= self.targets.len() {
            return;
        }
        let mut target = self.targets.remove(index);
        for profile in target.profiles.iter_mut() {
            let _ = profile.worker.sender.send(ControlMessage::Stop);
            if let Some(join) = profile.worker.join.take() {
                let _ = join.join();
            }
        }
        self.selected_target = self.selected_target.saturating_sub(1);
    }

    pub fn toggle_pause(&mut self, index: usize) {
        if let Some(target) = self.targets.get_mut(index) {
            target.paused = !target.paused;
            for profile in target.profiles.iter_mut() {
                let _ = profile
                    .worker
                    .sender
                    .send(ControlMessage::Pause(target.paused));
            }
        }
    }

    pub fn apply_sample(&mut self, sample: ProbeSample) {
        let key = ProfileKey {
            target_id: sample.target_id,
            profile_id: sample.profile_id,
        };

        if let Some(target) = self
            .targets
            .iter_mut()
            .find(|t| t.config.id == sample.target_id)
        {
            if let Some(remote) = sample.remote {
                target.last_ip = Some(remote.ip());
            }
            if let Some(profile) = target
                .profiles
                .iter_mut()
                .find(|p| p.config.id == sample.profile_id)
            {
                profile.last_sample = Some(sample.clone());
                profile.last_error = match &sample.result {
                    crate::probe::ProbeResult::Ok => None,
                    crate::probe::ProbeResult::Err(err) => Some(err.kind),
                };
                let max_points = target.config.sampling.max_points_per_window;
                self.metrics.push_sample(key, sample, max_points);
            }
        }
    }

    pub fn update_target_config(&mut self, index: usize, updated: TargetConfig) {
        if let Some(target) = self.targets.get_mut(index) {
            target.config = updated.clone();
            for profile in target.profiles.iter_mut() {
                let _ = profile
                    .worker
                    .sender
                    .send(ControlMessage::UpdateTarget(Box::new(updated.clone())));
            }
        }
    }

    pub fn update_profile_config(
        &mut self,
        target_index: usize,
        profile_index: usize,
        updated: ProfileConfig,
    ) {
        if let Some(target) = self.targets.get_mut(target_index) {
            if let Some(profile) = target.profiles.get_mut(profile_index) {
                profile.config = updated.clone();
                let _ = profile
                    .worker
                    .sender
                    .send(ControlMessage::UpdateProfile(Box::new(updated)));
            }
        }
    }

    pub fn cycle_window(&mut self) {
        let windows = &self.global.windows;
        if let Some(idx) = windows.iter().position(|w| *w == self.window) {
            let next = (idx + 1) % windows.len();
            self.window = windows[next];
        }
    }

    pub fn cycle_pane_mode(&mut self, index: usize) {
        if let Some(target) = self.targets.get_mut(index) {
            target.pane_mode = target.pane_mode.cycle();
        }
    }

    pub fn toggle_metric(&mut self, metric: MetricKind) {
        if self.selected_metrics.contains(&metric) {
            self.selected_metrics.remove(&metric);
        } else {
            self.selected_metrics.insert(metric);
        }
        if let Some(metric) = self.selected_metrics.iter().next().copied() {
            self.selected_metric = metric;
        }
    }

    pub fn selected_target(&self) -> Option<&TargetRuntime> {
        self.targets.get(self.selected_target)
    }

    pub fn selected_target_mut(&mut self) -> Option<&mut TargetRuntime> {
        self.targets.get_mut(self.selected_target)
    }

    pub fn target_aggregate(
        &self,
        target: &TargetRuntime,
        profile: &ProfileRuntime,
    ) -> WindowedAggregate {
        self.metrics.windowed_aggregate(
            ProfileKey {
                target_id: target.config.id,
                profile_id: profile.config.id,
            },
            self.window,
            &target.config.sampling,
            self.global.link_capacity_mbps,
        )
    }

    pub fn target_summary(&self, target: &TargetRuntime) -> GlobalSummary {
        let mut summary = GlobalSummary::default();
        for profile in &target.profiles {
            let aggregate = self.target_aggregate(target, profile);
            // ProbeLossRate's n contains total samples (success + error)
            if let Some(loss_stats) = aggregate.by_metric.get(&MetricKind::ProbeLossRate) {
                summary.samples += loss_stats.n;
            }
            if let Some(total_stats) = aggregate.by_metric.get(&MetricKind::Total) {
                summary.requests += total_stats.n;
            }
            for (kind, count) in &aggregate.error_breakdown {
                *summary.errors.entry(*kind).or_insert(0) += count;
            }
        }
        let total_errors: u64 = summary.errors.values().sum();
        summary.successes = summary.requests.saturating_sub(total_errors);
        summary.timeouts = summary
            .errors
            .iter()
            .filter(|(kind, _)| kind.is_timeout())
            .map(|(_, count)| *count)
            .sum();
        summary
    }
}

pub fn parse_target_url(input: &str) -> Option<Url> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        Url::parse(trimmed).ok()
    } else {
        Url::parse(&format!("https://{trimmed}")).ok()
    }
}

pub fn parse_profile_specs(input: &str) -> Vec<ProfileConfig> {
    let mut profiles = Vec::new();
    for raw in input.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(profile) = parse_profile_spec(raw) {
            profiles.push(profile);
        }
    }
    if profiles.is_empty() {
        default_profiles()
    } else {
        profiles
    }
}

fn parse_profile_spec(input: &str) -> Option<ProfileConfig> {
    let mut http = None;
    let mut tls = None;
    let mut reuse = None;
    let mut method = None;
    let mut max_bytes = None;

    for token in input.split('+').map(str::trim) {
        match token {
            "h1" | "http1" | "http1.1" => http = Some(crate::config::HttpVersion::H1),
            "h2" | "http2" => http = Some(crate::config::HttpVersion::H2),
            "tls12" | "tls1.2" => tls = Some(crate::config::TlsVersion::Tls12),
            "tls13" | "tls1.3" => tls = Some(crate::config::TlsVersion::Tls13),
            "warm" => reuse = Some(crate::config::ConnReusePolicy::Warm),
            "cold" => reuse = Some(crate::config::ConnReusePolicy::Cold),
            "head" => method = Some(crate::config::ProbeMethod::Head),
            "get" => method = Some(crate::config::ProbeMethod::Get),
            _ => {
                if let Ok(bytes) = token.parse::<u32>() {
                    max_bytes = Some(bytes);
                }
            }
        }
    }

    Some(ProfileConfig::new(
        input,
        http.unwrap_or(crate::config::HttpVersion::H2),
        tls.unwrap_or(crate::config::TlsVersion::Tls13),
        reuse.unwrap_or(crate::config::ConnReusePolicy::Warm),
        method.unwrap_or(crate::config::ProbeMethod::Get),
        max_bytes.unwrap_or(4096),
    ))
}

pub fn apply_edit_command(target: &TargetRuntime, input: &str) -> Option<TargetConfig> {
    let mut updated = target.config.clone();
    let mut modified = false;
    for token in input.split_whitespace() {
        if let Some(value) = token.strip_prefix("interval=") {
            if let Some(duration) = parse_duration(value) {
                updated.interval = duration;
                modified = true;
            }
        } else if let Some(value) = token.strip_prefix("timeout=") {
            if let Some(duration) = parse_duration(value) {
                updated.timeout_total = duration;
                modified = true;
            }
        } else if let Some(value) = token.strip_prefix("dns=") {
            match value {
                "on" | "true" => {
                    updated.dns_enabled = true;
                    modified = true;
                }
                "off" | "false" => {
                    updated.dns_enabled = false;
                    modified = true;
                }
                _ => {}
            }
        }
    }

    if modified {
        Some(updated)
    } else {
        None
    }
}

fn parse_duration(input: &str) -> Option<std::time::Duration> {
    if let Some(value) = input.strip_suffix("ms") {
        value
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_millis)
    } else if let Some(value) = input.strip_suffix('s') {
        value
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_secs)
    } else {
        input
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn parse_profile_spec_accepts_tokens() {
        let profiles = parse_profile_specs("h1+tls12+cold+head+128");
        assert_eq!(profiles.len(), 1);
        let profile = &profiles[0];
        assert_eq!(profile.http, crate::config::HttpVersion::H1);
        assert_eq!(profile.tls, crate::config::TlsVersion::Tls12);
        assert_eq!(profile.conn_reuse, crate::config::ConnReusePolicy::Cold);
        assert_eq!(profile.method, crate::config::ProbeMethod::Head);
        assert_eq!(profile.max_read_bytes, 128);
    }

    #[test]
    fn apply_edit_command_updates_target() {
        let url = Url::parse("https://google.com").unwrap();
        let target = TargetRuntime {
            config: TargetConfig::new(url, default_profiles()),
            paused: false,
            last_ip: None,
            profiles: Vec::new(),
            view_mode: ProfileViewMode::Single,
            selected_profile: 0,
            pane_mode: TargetPaneMode::Split,
        };

        let updated =
            apply_edit_command(&target, "interval=3s timeout=7s dns=off").expect("should update");
        assert_eq!(updated.interval, std::time::Duration::from_secs(3));
        assert_eq!(updated.timeout_total, std::time::Duration::from_secs(7));
        assert!(!updated.dns_enabled);
    }

    #[test]
    fn parse_target_url_adds_default_scheme() {
        let url = parse_target_url("google.com").expect("url should parse");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("google.com"));
    }

    #[test]
    fn parse_target_url_accepts_host_and_port() {
        let url = parse_target_url("localhost:8080").expect("url should parse");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.port_or_known_default(), Some(8080));
    }

    #[test]
    fn parse_target_url_rejects_empty_input() {
        assert!(parse_target_url("   ").is_none());
    }

    #[test]
    fn parse_duration_accepts_millis_and_seconds() {
        assert_eq!(parse_duration("150ms"), Some(Duration::from_millis(150)));
        assert_eq!(parse_duration("2s"), Some(Duration::from_secs(2)));
        assert_eq!(parse_duration("5"), Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_duration_rejects_invalid_values() {
        assert!(parse_duration("invalid").is_none());
    }

    #[test]
    fn apply_edit_command_returns_none_when_no_updates() {
        let url = Url::parse("https://google.com").unwrap();
        let target = TargetRuntime {
            config: TargetConfig::new(url, default_profiles()),
            paused: false,
            last_ip: None,
            profiles: Vec::new(),
            view_mode: ProfileViewMode::Single,
            selected_profile: 0,
            pane_mode: TargetPaneMode::Split,
        };

        assert!(apply_edit_command(&target, "foo=bar dns=maybe").is_none());
    }
}
