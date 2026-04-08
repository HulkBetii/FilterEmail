#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod processor;

use processor::{process_file_core, ErrorPayload};
use std::path::Path;
use tauri::{AppHandle, Emitter};

#[tauri::command(rename_all = "snake_case")]
async fn process_file(
    app: AppHandle,
    file_path: String,
    output_dir: String,
) -> Result<(), String> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || process_file_impl(app_handle, file_path, output_dir))
        .await
        .map_err(|error| error.to_string())?
}

fn process_file_impl(app: AppHandle, file_path: String, output_dir: String) -> Result<(), String> {
    let input_path = Path::new(&file_path);
    let output_path = Path::new(&output_dir);
    let payload = process_file_core(input_path, output_path, |payload, event_name| {
        app.emit(event_name, payload).map_err(|error| error.to_string())
    })
    .map_err(|error| emit_error_and_return(&app, error))?;

    app.emit("processing-complete", payload)
        .map_err(|error| error.to_string())
}

fn emit_error_and_return(app: &AppHandle, payload: ErrorPayload) -> String {
    let _ = app.emit("processing-error", payload.clone());
    payload.message_en
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![process_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
