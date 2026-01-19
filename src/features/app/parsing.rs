use super::state::TargetRuntime;
use crate::config::{
    ConnReusePolicy, HttpVersion, ProbeMethod, ProfileConfig, TargetConfig, TlsVersion,
    default_profiles_for_capabilities,
};
use crate::probe_engine::detect_tls13_support;

pub use crate::common::net::parse_target_url;

pub fn parse_profile_specs(input: &str) -> Vec<ProfileConfig> {
    let mut profiles = Vec::new();
    for raw in input.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(profile) = parse_profile_spec(raw) {
            profiles.push(profile);
        }
    }
    if profiles.is_empty() {
        // Auto-detect TLS 1.3 support and include it if available
        default_profiles_for_capabilities(detect_tls13_support())
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
            "h1" | "http1" | "http1.1" => http = Some(HttpVersion::H1),
            "h2" | "http2" => http = Some(HttpVersion::H2),
            "tls12" | "tls1.2" => tls = Some(TlsVersion::Tls12),
            "tls13" | "tls1.3" => tls = Some(TlsVersion::Tls13),
            "warm" => reuse = Some(ConnReusePolicy::Warm),
            "cold" => reuse = Some(ConnReusePolicy::Cold),
            "head" => method = Some(ProbeMethod::Head),
            "get" => method = Some(ProbeMethod::Get),
            _ => {
                if let Ok(bytes) = token.parse::<u32>() {
                    max_bytes = Some(bytes);
                }
            }
        }
    }

    Some(ProfileConfig::new(
        input,
        http.unwrap_or(HttpVersion::H2),
        tls.unwrap_or(TlsVersion::Tls13),
        reuse.unwrap_or(ConnReusePolicy::Warm),
        method.unwrap_or(ProbeMethod::Get),
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

    if modified { Some(updated) } else { None }
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
    use crate::app::{MetricsCategory, ProfileViewMode, TargetPaneMode};
    use std::time::Duration;
    use url::Url;

    #[test]
    fn parse_profile_spec_accepts_tokens() {
        let profiles = parse_profile_specs("h1+tls12+cold+head+128");
        assert_eq!(profiles.len(), 1);
        let profile = &profiles[0];
        assert_eq!(profile.http, HttpVersion::H1);
        assert_eq!(profile.tls, TlsVersion::Tls12);
        assert_eq!(profile.conn_reuse, ConnReusePolicy::Cold);
        assert_eq!(profile.method, ProbeMethod::Head);
        assert_eq!(profile.max_read_bytes, 128);
    }

    #[test]
    fn apply_edit_command_updates_target() {
        let url = Url::parse("https://google.com").unwrap();
        let target = TargetRuntime {
            config: TargetConfig::new(url, default_profiles_for_capabilities(false)),
            paused: false,
            last_ip: None,
            profiles: Vec::new(),
            view_mode: ProfileViewMode::Single,
            selected_profile: 0,
            pane_mode: TargetPaneMode::Split,
            metrics_category: MetricsCategory::default(),
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
            config: TargetConfig::new(url, default_profiles_for_capabilities(false)),
            paused: false,
            last_ip: None,
            profiles: Vec::new(),
            view_mode: ProfileViewMode::Single,
            selected_profile: 0,
            pane_mode: TargetPaneMode::Split,
            metrics_category: MetricsCategory::default(),
        };

        assert!(apply_edit_command(&target, "foo=bar dns=maybe").is_none());
    }
}
