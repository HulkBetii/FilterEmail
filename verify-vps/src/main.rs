mod cache;
mod catch_all;
mod rate_limiter;
mod smtp;

use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::post,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::smtp::{SmtpProbeResult, SmtpStatus, SmtpVerifier};

#[derive(Clone)]
struct AppState {
    api_key: Arc<String>,
    verifier: Arc<SmtpVerifier>,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug, Deserialize)]
struct VerifySmtpRequest {
    domains: Vec<String>,
    emails: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct VerifySmtpResponse {
    results: HashMap<String, VerifySmtpResult>,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct VerifySmtpResult {
    status: SmtpStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    mx_host: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let api_key = env::var("API_KEY").context("Missing API_KEY")?;
    let from_domain = env::var("SMTP_FROM_DOMAIN").context("Missing SMTP_FROM_DOMAIN")?;
    let timeout_secs = env::var("SMTP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(8);
    let max_concurrent = env::var("MAX_CONCURRENT_SMTP")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(30);

    let state = AppState {
        api_key: Arc::new(api_key),
        verifier: Arc::new(SmtpVerifier::new(
            from_domain,
            Duration::from_secs(timeout_secs.max(1)),
        )),
        semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
    };

    let app = Router::new()
        .route("/verify/smtp", post(verify_smtp))
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse().context("Invalid BIND_ADDR")?;
    let cert_path = env::var("TLS_CERT_PATH").ok();
    let key_path = env::var("TLS_KEY_PATH").ok();

    match (cert_path, key_path) {
        (Some(cert), Some(key)) => {
            let config = RustlsConfig::from_pem_file(cert, key).await?;
            axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service())
                .await?;
        }
        _ => {
            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}

async fn verify_smtp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<VerifySmtpRequest>,
) -> Result<Json<VerifySmtpResponse>, (StatusCode, String)> {
    authorize(&headers, &state.api_key)?;
    let started = std::time::Instant::now();

    let mut join_set = tokio::task::JoinSet::new();
    for domain in payload.domains {
        let verifier = Arc::clone(&state.verifier);
        let semaphore = Arc::clone(&state.semaphore);
        let email = payload
            .emails
            .get(&domain)
            .cloned()
            .unwrap_or_else(|| format!("postmaster@{domain}"));

        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let result = verifier.verify_email(&domain, &email).await;
            (domain, result)
        });
    }

    let mut results = HashMap::new();
    while let Some(result) = join_set.join_next().await {
        let (domain, smtp_result) = result.map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("SMTP task failed: {error}"),
            )
        })?;
        results.insert(domain, to_response_item(smtp_result));
    }

    Ok(Json(VerifySmtpResponse {
        results,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }))
}

fn authorize(headers: &HeaderMap, expected_api_key: &str) -> Result<(), (StatusCode, String)> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        ));
    };
    let Ok(value) = value.to_str() else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header".to_string(),
        ));
    };
    let expected = format!("Bearer {expected_api_key}");
    if value != expected {
        return Err((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()));
    }
    Ok(())
}

fn to_response_item(result: SmtpProbeResult) -> VerifySmtpResult {
    VerifySmtpResult {
        status: result.status,
        mx_host: result.mx_host,
    }
}
