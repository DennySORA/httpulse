use std::fmt;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

pub type TargetId = Uuid;
pub type ProfileId = Uuid;

#[derive(Clone, Debug)]
pub struct GlobalConfig {
    pub ui_refresh_hz: u16,
    pub default_window: WindowSpec,
    pub windows: Vec<WindowSpec>,
    pub link_capacity_mbps: Option<f64>,
    pub ebpf_enabled: bool,
    pub ebpf_mode: EbpfMode,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct TimeoutBreakdown {
    pub dns: Duration,
    pub connect: Duration,
    pub tls: Duration,
    pub ttfb: Duration,
    pub read: Duration,
}

#[derive(Clone, Debug)]
pub struct SamplingConfig {
    pub max_points_per_window: usize,
    pub histogram: HistogramConfig,
}

#[derive(Clone, Debug)]
pub struct HistogramConfig {
    pub latency_low_ms: u64,
    pub latency_high_ms: u64,
    pub sigfig: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum HttpVersion {
    H1,
    H2,
}

#[derive(Clone, Copy, Debug)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

#[derive(Clone, Copy, Debug)]
pub enum ConnReusePolicy {
    Warm,
    Cold,
}

#[derive(Clone, Copy, Debug)]
pub enum ProbeMethod {
    Head,
    Get,
}

#[derive(Clone, Copy, Debug)]
pub enum EbpfMode {
    Off,
    Minimal,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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

#[derive(Clone, Eq, PartialEq)]
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
