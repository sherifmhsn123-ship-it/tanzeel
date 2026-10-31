use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use tanzeel_core::{validate_settings, AppSettings, Language, SettingsError, Theme};
use thiserror::Error;

pub const DATABASE_FILE_NAME: &str = "tanzeel.db";

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/0001_settings.sql"))];

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid settings: {0}")]
    Settings(#[from] SettingsError),
    #[error("could not prepare the database directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("stored settings are invalid: {0}")]
    CorruptSettings(String),
}

pub struct Storage {
    connection: Connection,
}

impl Storage {
    /// Opens (or creates) the database file and applies pending migrations.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    /// Opens a transient in-memory database, mainly for tests.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StorageError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let mut storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        let transaction = self.connection.transaction()?;
        for (version, sql) in MIGRATIONS {
            let applied = transaction
                .query_row(
                    "SELECT version FROM schema_migrations WHERE version = ?1",
                    params![version],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;

            if applied.is_none() {
                transaction.execute_batch(sql)?;
                transaction.execute(
                    "INSERT INTO schema_migrations (version) VALUES (?1)",
                    params![version],
                )?;
            }
        }
        transaction.commit()?;

        Ok(())
    }

    /// Returns the stored settings, or defaults when nothing was saved yet.
    pub fn get_settings(&self) -> Result<AppSettings, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT language, theme, download_dir FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;

        let Some((language, theme, download_dir)) = row else {
            return Ok(AppSettings::default());
        };

        let settings = AppSettings {
            language: parse_language(&language)?,
            theme: parse_theme(&theme)?,
            download_dir,
        };
        validate_settings(&settings)?;
        Ok(settings)
    }

    /// Validates and stores the settings in a single row.
    pub fn save_settings(&mut self, settings: &AppSettings) -> Result<(), StorageError> {
        validate_settings(settings)?;

        self.connection.execute(
            "INSERT INTO settings (id, language, theme, download_dir, updated_at)
             VALUES (1, ?1, ?2, ?3, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                language = excluded.language,
                theme = excluded.theme,
                download_dir = excluded.download_dir,
                updated_at = excluded.updated_at",
            params![
                language_code(settings.language),
                theme_code(settings.theme),
                settings.download_dir
            ],
        )?;

        Ok(())
    }
}

fn language_code(language: Language) -> &'static str {
    match language {
        Language::Ar => "ar",
        Language::En => "en",
    }
}

fn theme_code(theme: Theme) -> &'static str {
    match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
        Theme::System => "system",
    }
}

fn parse_language(value: &str) -> Result<Language, StorageError> {
    match value {
        "ar" => Ok(Language::Ar),
        "en" => Ok(Language::En),
        other => Err(StorageError::CorruptSettings(format!(
            "unexpected language {other:?}"
        ))),
    }
}

fn parse_theme(value: &str) -> Result<Theme, StorageError> {
    match value {
        "light" => Ok(Theme::Light),
        "dark" => Ok(Theme::Dark),
        "system" => Ok(Theme::System),
        other => Err(StorageError::CorruptSettings(format!(
            "unexpected theme {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_storage() -> (tempfile::TempDir, Storage) {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let storage = Storage::open(&directory.path().join("nested").join(DATABASE_FILE_NAME))
            .expect("storage should open");
        (directory, storage)
    }

    fn applied_migration_count(storage: &Storage) -> i64 {
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration table should be readable")
    }

    #[test]
    fn creates_the_database_and_applies_migrations() {
        let (_directory, storage) = temporary_storage();

        assert_eq!(applied_migration_count(&storage), 1);
        assert_eq!(
            storage.get_settings().expect("defaults should load"),
            AppSettings::default()
        );
    }

    #[test]
    fn saves_and_reads_settings() {
        let (_directory, mut storage) = temporary_storage();
        let settings = AppSettings {
            language: Language::En,
            theme: Theme::Dark,
            download_dir: "C:\\Users\\example\\Downloads".to_string(),
        };

        storage
            .save_settings(&settings)
            .expect("valid settings should save");

        assert_eq!(
            storage.get_settings().expect("settings should load"),
            settings
        );
    }

    #[test]
    fn keeps_settings_after_reopening_the_database() {
        let (directory, mut storage) = temporary_storage();
        let settings = AppSettings {
            language: Language::En,
            theme: Theme::Light,
            download_dir: String::new(),
        };
        storage
            .save_settings(&settings)
            .expect("valid settings should save");
        drop(storage);

        let reopened = Storage::open(&directory.path().join("nested").join(DATABASE_FILE_NAME))
            .expect("storage should reopen");

        assert_eq!(
            reopened.get_settings().expect("settings should load"),
            settings
        );
        assert_eq!(applied_migration_count(&reopened), 1);
    }

    #[test]
    fn rejects_invalid_settings_without_changing_stored_values() {
        let (_directory, mut storage) = temporary_storage();
        let valid = AppSettings {
            language: Language::Ar,
            theme: Theme::System,
            download_dir: String::new(),
        };
        storage
            .save_settings(&valid)
            .expect("valid settings should save");

        let invalid = AppSettings {
            download_dir: "Downloads".to_string(),
            ..valid.clone()
        };
        let error = storage
            .save_settings(&invalid)
            .expect_err("relative directory should be rejected");

        assert!(matches!(error, StorageError::Settings(_)));
        assert_eq!(
            storage
                .get_settings()
                .expect("stored settings should remain"),
            valid
        );
    }

    #[test]
    fn rejects_control_characters_in_download_directory() {
        let (_directory, mut storage) = temporary_storage();
        let invalid = AppSettings {
            download_dir: "C:\\Downloads\nmalicious".to_string(),
            ..AppSettings::default()
        };

        let error = storage
            .save_settings(&invalid)
            .expect_err("control characters should be rejected");

        assert!(matches!(error, StorageError::Settings(_)));
    }

    #[test]
    fn reports_corrupt_stored_language() {
        let (_directory, storage) = temporary_storage();
        storage
            .connection
            .pragma_update(None, "ignore_check_constraints", "ON")
            .expect("check constraints should be disabled for test setup");
        storage
            .connection
            .execute(
                "INSERT INTO settings (id, language, theme, download_dir) VALUES (1, 'fr', 'system', '')",
                [],
            )
            .expect("corrupt row should be inserted");

        let error = storage
            .get_settings()
            .expect_err("unknown language should be reported");

        assert!(matches!(error, StorageError::CorruptSettings(_)));
    }

    #[test]
    fn enforces_a_single_settings_row() {
        let (_directory, storage) = temporary_storage();

        let result = storage.connection.execute(
            "INSERT INTO settings (id, language, theme, download_dir) VALUES (2, 'ar', 'system', '')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn migrations_are_idempotent() {
        let (_directory, mut storage) = temporary_storage();
        storage
            .migrate()
            .expect("running migrations again should be safe");
        storage
            .migrate()
            .expect("running migrations a third time should be safe");

        assert_eq!(applied_migration_count(&storage), 1);
    }
}
