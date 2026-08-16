use std::fmt::Display;
use std::io::Cursor;
use std::process::exit;

use anyhow::{Error, Result};
use clap::Parser;
use log::{LevelFilter, debug, error, warn};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use systemd_journal_logger::{JournalLog, connected_to_journal};
use tiny_http::{Response, Server};

use crate::cli::Cli;
use crate::metrics::QBitMetrics;

mod cli;
mod metrics;

fn main() {
    init_logging().expect("Failed to init logging system");

    let args = Cli::parse();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to start Tokio runtime");

    rt.block_on(serve(args)).map_err(|e| {
        error!("Failed to start application: {e}");
        exit(1);
    });
}

async fn serve(args: Cli) -> Result<()> {
    let mut registry = Registry::default();

    let qbit_metrics = QBitMetrics::new(
        &mut registry,
        args.qbittorrent_endpoint,
        args.qbittorrent_username,
        args.qbittorrent_password,
    )
    .await?;

    let address = ("0.0.0.0", args.exporter_port);
    let server = Server::http(address).map_err(Error::from_boxed)?;

    for request in server.incoming_requests() {
        debug!(
            "Received request {:?} {:?}",
            request.method(),
            request.url()
        );

        let response = match qbit_metrics.update_metrics().await {
            Ok(_) => prepare_response(&registry),
            Err(e) => error_response("Failed to refresh metrics", e),
        };

        request
            .respond(response)
            .unwrap_or_else(|e| warn!("Failed to send response: {e}"));
    }

    Ok(())
}

fn init_logging() -> Result<()> {
    let default_log_level = LevelFilter::Debug;

    if connected_to_journal() {
        JournalLog::new()?
            .with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))])
            .install()?;

        // rely on the same configuration approach as env_logger for consistency
        let log_level: String = std::env::var("RUST_LOG").unwrap_or(default_log_level.to_string());
        match log_level.parse() {
            Ok(level) => log::set_max_level(level),
            _ => {
                log::set_max_level(default_log_level);
                warn!(
                    "Invalid RUST_LOG value provided: '{log_level}'. Falling back to {default_log_level} level"
                );
            }
        }
    } else {
        let env = env_logger::Env::default().default_filter_or(default_log_level.to_string());
        env_logger::try_init_from_env(env)?;
    }

    Ok(())
}

fn prepare_response(registry: &Registry) -> Response<Cursor<Vec<u8>>> {
    let mut buffer = String::new();
    match encode(&mut buffer, registry) {
        Ok(_) => Response::from_string(buffer),
        Err(e) => error_response("Failed to encode metrics", e),
    }
}

fn error_response(msg: &str, root_cause: impl Display) -> Response<Cursor<Vec<u8>>> {
    warn!("{msg}: {root_cause}");
    Response::from_string(msg).with_status_code(500)
}
