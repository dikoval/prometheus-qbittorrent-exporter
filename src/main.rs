use std::error::Error;
use std::io::Cursor;

use clap::Parser;
use env_logger::Env;
use log::{debug, warn};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use tiny_http::{Response, Server};

use crate::cli::Cli;
use crate::metrics::QBitMetrics;

mod metrics;
mod cli;

fn main() {
    let log_level = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(log_level).init();

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
    let server = Server::http(address)
        .expect("Failed to start HTTP server");

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
