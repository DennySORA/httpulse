use crate::app::{MetricsCategory, ProfileViewMode, TargetPaneMode};
use crate::config::{GlobalConfig, TargetConfig, WindowSpec};
use crate::metrics::MetricKind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;

const STATE_FILE_VERSION: &str = "1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedState {
    pub version: String,
    pub global_config: GlobalConfig,
    pub targets: Vec<PersistedTarget>,
    pub ui_state: PersistedUiState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedTarget {
    pub config: TargetConfig,
    pub view_mode: ProfileViewMode,
    pub selected_profile: usize,
    pub pane_mode: TargetPaneMode,
    pub metrics_category: MetricsCategory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedUiState {
    pub selected_target: usize,
    pub selected_metric: MetricKind,
    pub selected_metrics: HashSet<MetricKind>,
    pub window: WindowSpec,
}

impl Default for PersistedState {
    fn default() -> Self {
        let global = GlobalConfig::default();
        let mut selected_metrics = HashSet::new();
        selected_metrics.insert(MetricKind::Total);
        Self {
            version: STATE_FILE_VERSION.to_string(),
            global_config: global.clone(),
            targets: Vec::new(),
            ui_state: PersistedUiState {
                selected_target: 0,
                selected_metric: MetricKind::Total,
                selected_metrics,
                window: global.default_window,
            },
        }
    }
}

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("httpulse"))
}

fn state_file_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("state.json"))
}

pub fn load() -> PersistedState {
    let Some(path) = state_file_path() else {
        return PersistedState::default();
    };

    if !path.exists() {
        return PersistedState::default();
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<PersistedState>(&content).ok())
        .unwrap_or_default()
}

pub fn save(state: &PersistedState) -> io::Result<()> {
    let Some(dir) = config_dir() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine config directory",
        ));
    };

    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let Some(path) = state_file_path() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine state file path",
        ));
    };

    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_valid() {
        let state = PersistedState::default();
        assert_eq!(state.version, STATE_FILE_VERSION);
        assert!(state.targets.is_empty());
        assert!(state.ui_state.selected_metrics.contains(&MetricKind::Total));
    }

    #[test]
    fn state_serialization_roundtrip() {
        let state = PersistedState::default();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: PersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, state.version);
    }
}
