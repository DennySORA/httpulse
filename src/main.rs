use httpulse::app::{AppState, parse_target_url};
use httpulse::settings::{apply_global, load_from_cli};
use httpulse::storage;
use httpulse::ui::run_ui;

fn main() -> std::io::Result<()> {
    let settings = load_from_cli()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string()))?;

    let persisted = storage::load();

    let mut global = persisted.global_config.clone();
    apply_global(&settings, &mut global);

    let (sample_tx, sample_rx) = crossbeam_channel::unbounded();
    let mut app = AppState::new(global);

    let is_default_target =
        settings.targets.len() == 1 && settings.targets[0] == "https://google.com";
    let has_cli_targets = !settings.targets.is_empty() && !is_default_target;

    if has_cli_targets {
        for target in settings.targets {
            if let Some(url) = parse_target_url(&target) {
                app.add_target(url, None, sample_tx.clone());
            }
        }
    } else if !persisted.targets.is_empty() {
        app.restore_from_persisted(&persisted, sample_tx.clone());
    } else {
        for target in settings.targets {
            if let Some(url) = parse_target_url(&target) {
                app.add_target(url, None, sample_tx.clone());
            }
        }
    }

    run_ui(app, sample_rx, sample_tx)?;
    Ok(())
}
