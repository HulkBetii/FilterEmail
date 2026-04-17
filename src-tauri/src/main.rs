#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod processor;
mod smtp_client;
mod smtp_status;
mod smtp_verify;

use processor::{ErrorPayload, process_file_core};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::runtime::Handle;

#[tauri::command(rename_all = "snake_case")]
#[allow(clippy::too_many_arguments)]
async fn process_file(
    app: AppHandle,
    file_paths: Vec<String>,
    output_dir: String,
    target_domains: String,
    check_mx: bool,
    timeout_ms: u64,
    max_concurrent: usize,
    use_persistent_cache: bool,
    smtp_enabled: bool,
    vps_api_url: String,
    vps_api_key: String,
) -> Result<(), String> {
    let runtime = Handle::current();
    let persistent_cache_path = if use_persistent_cache {
        let cache_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?;
        Some(
            cache_dir
                .join("cache")
                .join("mx_cache.sqlite3")
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        runtime.block_on(process_file_impl(
            app,
            file_paths,
            output_dir,
            target_domains,
            check_mx,
            timeout_ms,
            max_concurrent,
            use_persistent_cache,
            persistent_cache_path,
            smtp_enabled,
            vps_api_url,
            vps_api_key,
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[allow(clippy::too_many_arguments)]
async fn process_file_impl(
    app: AppHandle,
    file_paths: Vec<String>,
    output_dir: String,
    target_domains: String,
    check_mx: bool,
    timeout_ms: u64,
    max_concurrent: usize,
    use_persistent_cache: bool,
    persistent_cache_path: Option<String>,
    smtp_enabled: bool,
    vps_api_url: String,
    vps_api_key: String,
) -> Result<(), String> {
    let output_path = Path::new(&output_dir);

    let domains_vec: Vec<String> = target_domains
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let payload = process_file_core(
        file_paths,
        output_path,
        domains_vec,
        check_mx,
        timeout_ms,
        max_concurrent,
        use_persistent_cache,
        persistent_cache_path.as_deref().map(Path::new),
        smtp_enabled,
        &vps_api_url,
        &vps_api_key,
        |payload, event_name| {
            app.emit(event_name, payload)
                .map_err(|error| error.to_string())
        },
    )
    .await
    .map_err(|error| emit_error_and_return(&app, error))?;

    app.emit("processing-complete", payload)
        .map_err(|error| error.to_string())
}

fn emit_error_and_return(app: &AppHandle, payload: ErrorPayload) -> String {
    let _ = app.emit("processing-error", payload.clone());
    payload.message_en
}

/// Try a TCP connection to a known mail server on port 25.
/// Returns true if outbound port 25 is reachable.
#[tauri::command]
async fn check_port_25() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        TcpStream::connect_timeout(
            &"gmail-smtp-in.l.google.com:25".parse().unwrap(),
            Duration::from_secs(4),
        )
        .is_ok()
    })
    .await
    .unwrap_or(false)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![process_file, check_port_25])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
