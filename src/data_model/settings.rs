use crate::config::EbpfMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub targets: Vec<String>,
    pub refresh_hz: u16,
    pub ebpf_mode: EbpfMode,
}
