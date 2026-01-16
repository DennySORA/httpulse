use clap::Parser;
use monitor_network::app::AppState;
use monitor_network::config::{EbpfMode, GlobalConfig};
use monitor_network::ui::run_ui;
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "monitor_network")]
#[command(about = "HTTP latency and quality monitor", long_about = None)]
struct Args {
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

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut global = GlobalConfig::default();
    global.ui_refresh_hz = args.refresh_hz;
    global.ebpf_mode = parse_ebpf_mode(&args.ebpf);
    global.ebpf_enabled = global.ebpf_mode != EbpfMode::Off;

    let (sample_tx, sample_rx) = crossbeam_channel::unbounded();
    let mut app = AppState::new(global);

    let targets = if args.target.is_empty() {
        vec!["https://example.com".to_string()]
    } else {
        args.target
    };

    for target in targets {
        if let Ok(url) = Url::parse(&target) {
            app.add_target(url, None, sample_tx.clone());
        }
    }

    run_ui(app, sample_rx, sample_tx)?;
    Ok(())
}

fn parse_ebpf_mode(value: &str) -> EbpfMode {
    match value {
        "minimal" => EbpfMode::Minimal,
        "full" => EbpfMode::Full,
        _ => EbpfMode::Off,
    }
}
