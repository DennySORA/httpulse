mod add;
mod confirm;
mod help;
mod normal;
mod settings;

pub(super) use add::handle_input_key;
pub(super) use confirm::handle_confirm_delete_key;
pub(super) use help::{handle_glossary_key, handle_help_key};
pub(super) use normal::handle_normal_key;
pub(super) use settings::{handle_settings_edit_key, handle_settings_key};
