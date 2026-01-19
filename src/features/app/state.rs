use crate::config::{GlobalConfig, ProfileConfig, TargetConfig, default_profiles_for_capabilities};
use crate::metrics::{MetricKind, WindowedAggregate};
use crate::metrics_aggregate::{MetricsStore, ProfileKey};
use crate::probe::{ProbeErrorKind, ProbeSample};
use crate::probe_engine::detect_tls13_support;
use crate::runtime::{ControlMessage, WorkerHandle, spawn_profile_worker};
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

/// Metrics category for tab-based navigation
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum MetricsCategory {
    #[default]
    Latency,
    Quality,
    Reliability,
    Throughput,
    Tcp,
}

impl MetricsCategory {
    pub fn label(self) -> &'static str {
        match self {
            MetricsCategory::Latency => "Latency",
            MetricsCategory::Quality => "Quality",
            MetricsCategory::Reliability => "Reliability",
            MetricsCategory::Throughput => "Throughput",
            MetricsCategory::Tcp => "TCP",
        }
    }

    pub fn next(self) -> Self {
        match self {
            MetricsCategory::Latency => MetricsCategory::Quality,
            MetricsCategory::Quality => MetricsCategory::Reliability,
            MetricsCategory::Reliability => MetricsCategory::Throughput,
            MetricsCategory::Throughput => MetricsCategory::Tcp,
            MetricsCategory::Tcp => MetricsCategory::Latency,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            MetricsCategory::Latency => MetricsCategory::Tcp,
            MetricsCategory::Quality => MetricsCategory::Latency,
            MetricsCategory::Reliability => MetricsCategory::Quality,
            MetricsCategory::Throughput => MetricsCategory::Reliability,
            MetricsCategory::Tcp => MetricsCategory::Throughput,
        }
    }

    pub const ALL: [MetricsCategory; 5] = [
        MetricsCategory::Latency,
        MetricsCategory::Quality,
        MetricsCategory::Reliability,
        MetricsCategory::Throughput,
        MetricsCategory::Tcp,
    ];
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
    /// Selected metrics category for tab-based navigation
    pub metrics_category: MetricsCategory,
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
        let profiles =
            profiles.unwrap_or_else(|| default_profiles_for_capabilities(detect_tls13_support()));
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
            metrics_category: MetricsCategory::default(),
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
        if let Some(target) = self.targets.get_mut(target_index)
            && let Some(profile) = target.profiles.get_mut(profile_index)
        {
            profile.config = updated.clone();
            let _ = profile
                .worker
                .sender
                .send(ControlMessage::UpdateProfile(Box::new(updated)));
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
