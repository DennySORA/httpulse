/// Minimum terminal width required (columns)
pub(super) const MIN_TERMINAL_WIDTH: u16 = 100;
/// Minimum terminal height required (rows)
pub(super) const MIN_TERMINAL_HEIGHT: u16 = 24;
pub(super) const GLOSSARY_PAGE_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SettingsField {
    UiRefreshHz,
    LinkCapacityMbps,
    TargetInterval,
    TargetTimeout,
    TargetDnsEnabled,
    TargetPane,
    TargetPaused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InputMode {
    Normal,
    AddTarget,
    Help,
    Glossary,
    Settings,
    SettingsEdit(SettingsField),
    ConfirmDelete,
}

pub(super) struct SettingsRow {
    pub(super) field: SettingsField,
    pub(super) scope: &'static str,
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) action: &'static str,
}

pub(super) struct SettingsState {
    pub(super) selected: usize,
    pub(super) notice: Option<String>,
}

impl SettingsState {
    pub(super) fn new() -> Self {
        Self {
            selected: 0,
            notice: None,
        }
    }

    pub(super) fn select_next(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected + 1) % total;
    }

    pub(super) fn select_prev(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
            return;
        }
        if self.selected == 0 {
            self.selected = total - 1;
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub(super) fn clamp(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
        } else if self.selected >= total {
            self.selected = total - 1;
        }
    }

    pub(super) fn clear_notice(&mut self) {
        self.notice = None;
    }
}

pub(super) fn parse_link_capacity_mbps(input: &str) -> Result<Option<f64>, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized = trimmed.to_ascii_lowercase();
    if normalized == "off" || normalized == "none" {
        return Ok(None);
    }

    let numeric = normalized.strip_suffix("mbps").unwrap_or(&normalized);
    let value = numeric.parse::<f64>().map_err(|_| "Invalid number")?;
    if value <= 0.0 {
        return Err("Value must be > 0");
    }

    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::parse_link_capacity_mbps;

    #[test]
    fn parse_link_capacity_allows_off_values() {
        assert_eq!(parse_link_capacity_mbps("").unwrap(), None);
        assert_eq!(parse_link_capacity_mbps("off").unwrap(), None);
        assert_eq!(parse_link_capacity_mbps("none").unwrap(), None);
    }

    #[test]
    fn parse_link_capacity_accepts_numbers() {
        assert_eq!(parse_link_capacity_mbps("100").unwrap(), Some(100.0));
        assert_eq!(parse_link_capacity_mbps("250.5").unwrap(), Some(250.5));
        assert_eq!(parse_link_capacity_mbps("42Mbps").unwrap(), Some(42.0));
    }

    #[test]
    fn parse_link_capacity_rejects_invalid() {
        assert!(parse_link_capacity_mbps("-1").is_err());
        assert!(parse_link_capacity_mbps("abc").is_err());
    }
}
