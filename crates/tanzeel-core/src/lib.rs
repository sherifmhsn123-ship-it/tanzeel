pub mod download;
pub mod settings;

pub use download::{
    is_private_or_local_host, is_private_or_local_ip, safe_filename, validate_download_url,
    UrlValidationError, MAX_FILE_NAME_LEN, MAX_URL_LEN,
};
pub use settings::{
    validate_settings, AppSettings, Language, SettingsError, Theme, MAX_DOWNLOAD_DIR_LEN,
};
