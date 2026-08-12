mod diary;

use std::path::PathBuf;

use tauri::Manager;

#[tauri::command]
fn list_entry_dates(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    diary::list_entry_dates(&entries_directory(&app)?).map_err(|error| error.to_string())
}

#[tauri::command]
fn read_entry(app: tauri::AppHandle, date: &str) -> Result<Option<String>, String> {
    diary::read_entry(&entries_directory(&app)?, date).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_entry(app: tauri::AppHandle, date: &str, content: &str) -> Result<(), String> {
    diary::save_entry(&entries_directory(&app)?, date, content).map_err(|error| error.to_string())
}

fn entries_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("entries"))
        .map_err(|error| format!("日記の保存先を取得できません: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_entry_dates,
            read_entry,
            save_entry
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
