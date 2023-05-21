use clap::{Parser};

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value="7071")]
    pub exporter_port: u16,

    #[arg(long, default_value="http://localhost:8080/")]
    pub qbittorrent_endpoint: String,
    #[arg(long, default_value="admin")]
    pub qbittorrent_username: String,
    #[arg(long, default_value="adminadmin")]
    pub qbittorrent_password: String
}
