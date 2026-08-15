use std::error::Error;
use std::io::Cursor;
use std::process::exit;

use clap::Parser;
use log::{LevelFilter, debug, error, warn};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use tiny_http::{Response, Server};
use systemd_journal_logger::{connected_to_journal, JournalLog};

use crate::cli::Cli;
use crate::metrics::QBitMetrics;

mod metrics;
mod cli;

fn main() {
    init_logging().expect("Failed to init logging system");

    let args = Cli::parse();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to start Tokio runtime");
    rt.block_on(serve(args));
}

async fn serve(args: Cli) {
    let mut registry = Registry::default();

    let qbit_metrics = QBitMetrics::new(
        &mut registry,
        args.qbittorrent_endpoint, args.qbittorrent_username, args.qbittorrent_password
    );

    let address = ("0.0.0.0", args.exporter_port);
    let server = Server::http(address).unwrap_or_else(|e| {
        error!("Failed to start HTTP server: {e}");
        exit(1);
    });

    for request in server.incoming_requests() {
        debug!("Received request {:?} {:?}", request.method(), request.url());

        let response = qbit_metrics.update_metrics().await
            .map_or_else(
                |e| encode_error(&e),
                |_| encode_metrics(&registry)
            );

        request.respond(response).expect("Failed to send response");
    }
}

fn init_logging() -> Result<(), Box<dyn Error>> {
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
                warn!("Invalid RUST_LOG value provided: '{log_level}'. Falling back to {default_log_level} level");
            }
        }
    } else {
        let env = env_logger::Env::default().default_filter_or(default_log_level.to_string());
        env_logger::try_init_from_env(env)?;
    }

    Ok(())
}

fn encode_error(error: &Box<dyn Error>) -> Response<Cursor<Vec<u8>>> {
    warn!("Request has failed with error {}", error);
    return Response::from_string(error.to_string())
        .with_status_code(500);
}

fn encode_metrics(registry: &Registry) -> Response<Cursor<Vec<u8>>> {
    let mut buffer = String::new();
    encode(&mut buffer, &registry).expect("Failed to encode metrics");

    return Response::from_string(buffer);
}
