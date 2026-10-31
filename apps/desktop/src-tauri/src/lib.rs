use std::sync::Mutex;

use tanzeel_core::AppSettings;
use tanzeel_storage::{Storage, DATABASE_FILE_NAME};
use tauri::{Manager, State};

struct AppState {
    storage: Mutex<Storage>,
}

fn storage_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| "settings storage is unavailable".to_string())?;

    storage.get_settings().map_err(storage_error)
}

#[tauri::command]
fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let mut storage = state
        .storage
        .lock()
        .map_err(|_| "settings storage is unavailable".to_string())?;

    storage.save_settings(&settings).map_err(storage_error)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let storage = Storage::open(&data_dir.join(DATABASE_FILE_NAME))?;
            app.manage(AppState {
                storage: Mutex::new(storage),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_settings, save_settings])
        .run(tauri::generate_context!())
        .expect("error while running Tanzeel");
}
