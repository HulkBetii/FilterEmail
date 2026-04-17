#[path = "../processor/mod.rs"]
mod processor;
#[path = "../smtp_client.rs"]
mod smtp_client;
#[path = "../smtp_status.rs"]
mod smtp_status;
#[path = "../smtp_verify.rs"]
mod smtp_verify;

use processor::ProcessingPayload;
use std::path::Path;

#[tokio::main]
async fn main() {
    let file_paths = vec!["/Users/sangspm/Downloads/emails.txt".to_string()];
    let output_path = Path::new("/tmp/email_filter_test");
    let target_domains = vec!["students.hcde.org".to_string(), "gmx.es".to_string()];
    let check_mx = false;

    println!("Testing without MX Check...");
    let result = processor::process_file_core(
        file_paths.clone(),
        output_path,
        target_domains.clone(),
        check_mx,
        1_500,
        25,
        false,
        None,
        false,
        "",
        "",
        |payload: ProcessingPayload, event| {
            println!("Event {}: {:.1}%", event, payload.progress_percent);
            Ok(())
        },
    )
    .await;
    println!("{:#?}", result);

    println!("\nTesting WITH MX Check...");
    let result_mx = processor::process_file_core(
        file_paths,
        output_path,
        target_domains,
        true,
        1_500,
        25,
        false,
        None,
        false,
        "",
        "",
        |_payload, _event| Ok(()),
    )
    .await;
    println!("{:#?}", result_mx);
}
