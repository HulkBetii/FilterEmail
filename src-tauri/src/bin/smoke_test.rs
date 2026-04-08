#[path = "../processor.rs"]
mod processor;

use processor::process_file_core;
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
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

    match process_file_core(&input_file, &output_dir, |payload, event_name| {
        println!(
            "{event_name}: lines={}, invalid={}, public={}, edu={}, custom={}, progress={:.1}%",
            payload.processed_lines,
            payload.invalid,
            payload.public,
            payload.edu,
            payload.custom,
            payload.progress_percent
        );
        Ok(())
    }) {
        Ok(payload) => {
            println!(
                "complete: lines={}, invalid={}, public={}, edu={}, custom={}, output_dir={}",
                payload.processed_lines,
                payload.invalid,
                payload.public,
                payload.edu,
                payload.custom,
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
