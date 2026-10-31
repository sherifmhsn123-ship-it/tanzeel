use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum accepted length of the configured download directory.
pub const MAX_DOWNLOAD_DIR_LEN: usize = 1024;

/// Interface language. Arabic is the default and the product is Arabic-first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Ar,
    En,
}

/// Visual theme preference. `System` follows the operating system setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// Settings stored locally for the current user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub language: Language,
    pub theme: Theme,
    /// Empty means the operating system default Downloads folder.
    pub download_dir: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Ar,
            theme: Theme::System,
            download_dir: String::new(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettingsError {
    #[error("download directory is longer than {MAX_DOWNLOAD_DIR_LEN} characters")]
    DownloadDirTooLong,
    #[error("download directory contains a control character")]
    DownloadDirControlCharacter,
    #[error("download directory has leading or trailing whitespace")]
    DownloadDirWhitespace,
    #[error("download directory must be an absolute path or empty")]
    DownloadDirNotAbsolute,
}

/// Validates user-controlled settings before they are stored or used.
pub fn validate_settings(settings: &AppSettings) -> Result<(), SettingsError> {
    let directory = settings.download_dir.as_str();

    if directory.chars().count() > MAX_DOWNLOAD_DIR_LEN {
        return Err(SettingsError::DownloadDirTooLong);
    }
    if directory.chars().any(char::is_control) {
        return Err(SettingsError::DownloadDirControlCharacter);
    }
    if directory.trim() != directory {
        return Err(SettingsError::DownloadDirWhitespace);
    }
    if !directory.is_empty() && !is_absolute_download_directory(directory) {
        return Err(SettingsError::DownloadDirNotAbsolute);
    }

    Ok(())
}

/// Tanzeel targets Windows, but tests run on Linux and macOS too. Accept a
/// Windows drive or UNC path on any host, and also accept a host-absolute path
/// so the storage tests can use the platform temporary directory.
fn is_absolute_download_directory(directory: &str) -> bool {
    is_windows_absolute(directory) || Path::new(directory).is_absolute()
}

fn is_windows_absolute(directory: &str) -> bool {
    let bytes = directory.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');

    let unc_absolute = directory
        .strip_prefix("\\\\")
        .and_then(|rest| rest.chars().next())
        .is_some_and(|first| first != '\\');

    drive_absolute || unc_absolute
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_arabic_first() {
        let settings = AppSettings::default();

        assert_eq!(settings.language, Language::Ar);
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.download_dir, "");
        assert_eq!(validate_settings(&settings), Ok(()));
    }

    #[test]
    fn accepts_absolute_download_directory() {
        let settings = AppSettings {
            download_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            ..AppSettings::default()
        };

        assert_eq!(validate_settings(&settings), Ok(()));
    }

    #[test]
    fn accepts_windows_drive_and_unc_paths_on_every_platform() {
        for directory in [
            "C:\\Users\\example\\Downloads",
            "D:/Downloads",
            "\\\\server\\share",
        ] {
            let settings = AppSettings {
                download_dir: directory.to_string(),
                ..AppSettings::default()
            };

            assert_eq!(
                validate_settings(&settings),
                Ok(()),
                "{directory} should be accepted"
            );
        }
    }

    #[test]
    fn rejects_relative_download_directory() {
        let settings = AppSettings {
            download_dir: "Downloads".to_string(),
            ..AppSettings::default()
        };

        assert_eq!(
            validate_settings(&settings),
            Err(SettingsError::DownloadDirNotAbsolute)
        );
    }

    #[test]
    fn rejects_overlong_download_directory() {
        let settings = AppSettings {
            download_dir: format!("C:\\{}", "a".repeat(MAX_DOWNLOAD_DIR_LEN)),
            ..AppSettings::default()
        };

        assert_eq!(
            validate_settings(&settings),
            Err(SettingsError::DownloadDirTooLong)
        );
    }

    #[test]
    fn rejects_control_characters() {
        let settings = AppSettings {
            download_dir: "C:\\Downloads\0hidden".to_string(),
            ..AppSettings::default()
        };

        assert_eq!(
            validate_settings(&settings),
            Err(SettingsError::DownloadDirControlCharacter)
        );
    }

    #[test]
    fn rejects_surrounding_whitespace() {
        let settings = AppSettings {
            download_dir: "  C:\\Downloads  ".to_string(),
            ..AppSettings::default()
        };

        assert_eq!(
            validate_settings(&settings),
            Err(SettingsError::DownloadDirWhitespace)
        );
    }

    #[test]
    fn serializes_with_camel_case_and_lowercase_enums() {
        let settings = AppSettings {
            language: Language::En,
            theme: Theme::Dark,
            download_dir: "C:\\Downloads".to_string(),
        };

        let json = serde_json::to_string(&settings).expect("settings should serialize");
        assert_eq!(
            json,
            r#"{"language":"en","theme":"dark","downloadDir":"C:\\Downloads"}"#
        );

        let parsed: AppSettings = serde_json::from_str(&json).expect("settings should deserialize");
        assert_eq!(parsed, settings);
    }

    #[test]
    fn rejects_unknown_fields_and_invalid_enums() {
        let unknown = r#"{"language":"ar","theme":"system","downloadDir":"","extra":true}"#;
        assert!(serde_json::from_str::<AppSettings>(unknown).is_err());

        let invalid = r#"{"language":"fr","theme":"system","downloadDir":""}"#;
        assert!(serde_json::from_str::<AppSettings>(invalid).is_err());
    }
}
