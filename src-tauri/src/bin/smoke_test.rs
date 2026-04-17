#[path = "../processor/mod.rs"]
mod processor;
#[path = "../smtp_client.rs"]
mod smtp_client;
#[path = "../smtp_status.rs"]
mod smtp_status;
#[path = "../smtp_verify.rs"]
mod smtp_verify;

use processor::{ProcessingPayload, process_file_core};
use std::{env, path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(input_file) = args.next() else {
        eprintln!("Usage: cargo run --bin smoke_test -- <input-file> <output-dir>");
        return ExitCode::FAILURE;
    };
    let Some(output_dir) = args.next() else {
        eprintln!("Usage: cargo run --bin smoke_test -- <input-file> <output-dir>");
        return ExitCode::FAILURE;
    };

    let input_file = PathBuf::from(input_file);
    let output_dir = PathBuf::from(output_dir);

    match process_file_core(
        vec![input_file.to_string_lossy().to_string()],
        &output_dir,
        Vec::new(),
        false,
        1_500,
        25,
        false,
        None,
        false,
        "",
        "",
        |payload: ProcessingPayload, event_name| {
        println!(
            "{event_name}: lines={}, invalid={}, public={}, edu={}, custom={}, inconclusive={}, progress={:.1}%",
            payload.processed_lines,
            payload.invalid,
            payload.public,
            payload.edu,
            payload.custom,
            payload.mx_inconclusive,
            payload.progress_percent
        );
        Ok(())
        },
    )
    .await
    {
        Ok(payload) => {
            println!(
                "complete: lines={}, invalid={}, public={}, edu={}, custom={}, dead={}, output_dir={}",
                payload.processed_lines,
                payload.invalid,
                payload.public,
                payload.edu,
                payload.custom,
                payload.mx_dead,
                payload.output_dir.unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error_en: {}", error.message_en);
            eprintln!("error_vi: {}", error.message_vi);
            ExitCode::FAILURE
        }
    }
}
