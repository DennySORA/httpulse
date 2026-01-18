use httpulse::app::{AppState, parse_target_url};
use httpulse::config::GlobalConfig;
use httpulse::settings::{apply_global, load_from_cli};
use httpulse::ui::run_ui;

fn main() -> std::io::Result<()> {
    let settings = load_from_cli()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string()))?;
    let mut global = GlobalConfig::default();
    apply_global(&settings, &mut global);

    let (sample_tx, sample_rx) = crossbeam_channel::unbounded();
    let mut app = AppState::new(global);

    for target in settings.targets {
        if let Some(url) = parse_target_url(&target) {
            app.add_target(url, None, sample_tx.clone());
        }
    }

    run_ui(app, sample_rx, sample_tx)?;
    Ok(())
}
