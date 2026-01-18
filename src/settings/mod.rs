use crate::config::{EbpfMode, GlobalConfig};
use crate::data_model::settings::AppSettings;
use clap::Parser;
use thiserror::Error;

const DEFAULT_TARGET: &str = "https://google.com";

#[derive(Parser, Debug)]
#[command(name = "httpulse")]
#[command(about = "Real-time HTTP latency and network quality monitor", long_about = None)]
pub struct CliArgs {
    /// Target URL to probe (repeatable)
    #[arg(short, long, value_name = "URL")]
    target: Vec<String>,

    /// UI refresh rate (Hz)
    #[arg(long, default_value_t = 10)]
    refresh_hz: u16,

    /// eBPF mode: off|minimal|full
    #[arg(long, default_value = "off")]
    ebpf: String,
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("ui refresh rate must be greater than zero (got {value})")]
    InvalidRefreshHz { value: u16 },
}

pub fn load_from_cli() -> Result<AppSettings, SettingsError> {
    let args = CliArgs::parse();
    from_args(args)
}

pub fn from_args(args: CliArgs) -> Result<AppSettings, SettingsError> {
    if args.refresh_hz == 0 {
        return Err(SettingsError::InvalidRefreshHz {
            value: args.refresh_hz,
        });
    }

    let targets = if args.target.is_empty() {
        vec![DEFAULT_TARGET.to_string()]
    } else {
        args.target
    };

    Ok(AppSettings {
        targets,
        refresh_hz: args.refresh_hz,
        ebpf_mode: EbpfMode::parse_cli(&args.ebpf),
    })
}

pub fn apply_global(settings: &AppSettings, global: &mut GlobalConfig) {
    global.ui_refresh_hz = settings.refresh_hz;
    global.ebpf_mode = settings.ebpf_mode;
    global.ebpf_enabled = global.ebpf_mode != EbpfMode::Off;
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TARGET, SettingsError, from_args};
    use crate::config::EbpfMode;

    #[test]
    fn from_args_defaults_target_and_ebpf_off() {
        let settings = from_args(super::CliArgs {
            target: Vec::new(),
            refresh_hz: 10,
            ebpf: "off".to_string(),
        })
        .expect("settings");

        assert_eq!(settings.targets, vec![DEFAULT_TARGET.to_string()]);
        assert_eq!(settings.refresh_hz, 10);
        assert_eq!(settings.ebpf_mode, EbpfMode::Off);
    }

    #[test]
    fn from_args_unknown_ebpf_defaults_off() {
        let settings = from_args(super::CliArgs {
            target: vec!["https://example.com".to_string()],
            refresh_hz: 10,
            ebpf: "unknown".to_string(),
        })
        .expect("settings");

        assert_eq!(settings.ebpf_mode, EbpfMode::Off);
    }

    #[test]
    fn from_args_rejects_zero_refresh_hz() {
        let err = from_args(super::CliArgs {
            target: Vec::new(),
            refresh_hz: 0,
            ebpf: "off".to_string(),
        })
        .expect_err("should error");

        match err {
            SettingsError::InvalidRefreshHz { value } => assert_eq!(value, 0),
        }
    }
}
