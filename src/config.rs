use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

pub type TargetId = Uuid;
pub type ProfileId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub ui_refresh_hz: u16,
    pub default_window: WindowSpec,
    pub windows: Vec<WindowSpec>,
    pub link_capacity_mbps: Option<f64>,
    pub ebpf_enabled: bool,
    pub ebpf_mode: EbpfMode,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            ui_refresh_hz: 10,
            default_window: WindowSpec::M15,
            windows: vec![
                WindowSpec::M1,
                WindowSpec::M5,
                WindowSpec::M15,
                WindowSpec::H1,
            ],
            link_capacity_mbps: None,
            ebpf_enabled: false,
            ebpf_mode: EbpfMode::Off,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub id: TargetId,
    pub url: Url,
    pub enabled: bool,
    pub dns_enabled: bool,
    pub interval: Duration,
    pub timeout_total: Duration,
    pub timeout_breakdown: Option<TimeoutBreakdown>,
    pub profiles: Vec<ProfileConfig>,
    pub sampling: SamplingConfig,
}

impl TargetConfig {
    pub fn new(url: Url, profiles: Vec<ProfileConfig>) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            enabled: true,
            dns_enabled: true,
            interval: Duration::from_secs(5),
            timeout_total: Duration::from_secs(10),
            timeout_breakdown: None,
            profiles,
            sampling: SamplingConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub id: ProfileId,
    pub name: String,
    pub http: HttpVersion,
    pub tls: TlsVersion,
    pub conn_reuse: ConnReusePolicy,
    pub method: ProbeMethod,
    pub max_read_bytes: u32,
    pub headers: Vec<(String, SecretString)>,
}

impl ProfileConfig {
    pub fn new(
        name: impl Into<String>,
        http: HttpVersion,
        tls: TlsVersion,
        conn_reuse: ConnReusePolicy,
        method: ProbeMethod,
        max_read_bytes: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            http,
            tls,
            conn_reuse,
            method,
            max_read_bytes,
            headers: Vec::new(),
        }
    }
}

pub fn default_profiles() -> Vec<ProfileConfig> {
    vec![
        ProfileConfig::new(
            "h2+tls13+warm",
            HttpVersion::H2,
            TlsVersion::Tls13,
            ConnReusePolicy::Warm,
            ProbeMethod::Get,
            4096,
        ),
        ProfileConfig::new(
            "h1+tls12+cold",
            HttpVersion::H1,
            TlsVersion::Tls12,
            ConnReusePolicy::Cold,
            ProbeMethod::Get,
            4096,
        ),
    ]
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TimeoutBreakdown {
    pub dns: Duration,
    pub connect: Duration,
    pub tls: Duration,
    pub ttfb: Duration,
    pub read: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SamplingConfig {
    pub max_points_per_window: usize,
    pub histogram: HistogramConfig,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            max_points_per_window: 1024,
            histogram: HistogramConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistogramConfig {
    pub latency_low_ms: u64,
    pub latency_high_ms: u64,
    pub sigfig: u8,
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            latency_low_ms: 1,
            latency_high_ms: 60_000,
            sigfig: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpVersion {
    H1,
    H2,
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpVersion::H1 => f.write_str("h1"),
            HttpVersion::H2 => f.write_str("h2"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl fmt::Display for TlsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsVersion::Tls12 => f.write_str("tls12"),
            TlsVersion::Tls13 => f.write_str("tls13"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnReusePolicy {
    Warm,
    Cold,
}

impl fmt::Display for ConnReusePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnReusePolicy::Warm => f.write_str("warm"),
            ConnReusePolicy::Cold => f.write_str("cold"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeMethod {
    Head,
    Get,
}

impl fmt::Display for ProbeMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeMethod::Head => f.write_str("head"),
            ProbeMethod::Get => f.write_str("get"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EbpfMode {
    Off,
    Minimal,
    Full,
}

impl EbpfMode {
    pub fn parse_cli(value: &str) -> Self {
        match value {
            "minimal" => EbpfMode::Minimal,
            "full" => EbpfMode::Full,
            _ => EbpfMode::Off,
        }
    }
}

impl fmt::Display for EbpfMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EbpfMode::Off => f.write_str("off"),
            EbpfMode::Minimal => f.write_str("minimal"),
            EbpfMode::Full => f.write_str("full"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowSpec {
    M1,
    M5,
    M15,
    H1,
}

impl WindowSpec {
    pub fn duration(self) -> Duration {
        match self {
            WindowSpec::M1 => Duration::from_secs(60),
            WindowSpec::M5 => Duration::from_secs(5 * 60),
            WindowSpec::M15 => Duration::from_secs(15 * 60),
            WindowSpec::H1 => Duration::from_secs(60 * 60),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WindowSpec::M1 => "1m",
            WindowSpec::M5 => "5m",
            WindowSpec::M15 => "15m",
            WindowSpec::H1 => "60m",
        }
    }
}

impl fmt::Display for WindowSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_spec_duration_matches_expected() {
        assert_eq!(WindowSpec::M1.duration(), Duration::from_secs(60));
        assert_eq!(WindowSpec::M5.duration(), Duration::from_secs(5 * 60));
        assert_eq!(WindowSpec::M15.duration(), Duration::from_secs(15 * 60));
        assert_eq!(WindowSpec::H1.duration(), Duration::from_secs(60 * 60));
    }

    #[test]
    fn window_spec_label_matches_expected() {
        assert_eq!(WindowSpec::M1.label(), "1m");
        assert_eq!(WindowSpec::M5.label(), "5m");
        assert_eq!(WindowSpec::M15.label(), "15m");
        assert_eq!(WindowSpec::H1.label(), "60m");
        assert_eq!(WindowSpec::H1.to_string(), "60m");
    }

    #[test]
    fn secret_string_redacts_debug_and_display() {
        let secret = SecretString::new("token-value");

        let debug_text = format!("{secret:?}");
        let display_text = secret.to_string();

        assert!(!debug_text.contains("token-value"));
        assert!(!display_text.contains("token-value"));
        assert_eq!(display_text, "[REDACTED]");
    }
}
