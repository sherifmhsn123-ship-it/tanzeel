pub mod settings;

pub use settings::{
    validate_settings, AppSettings, Language, SettingsError, Theme, MAX_DOWNLOAD_DIR_LEN,
};
