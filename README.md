# Prometheus QBittorrent Exporter

Prometheus Exporter for QBittorrent torrent client.

## Metrics

Next metrics are reported by this exporter:

| Metric name                          | Type    | Description                                                      |
|--------------------------------------|---------|------------------------------------------------------------------|
| `qbittorrent_status`                 | gauge   | Current status (connected/not connected) of QBittorrent instance |
| `qbittorrent_dht_nodes_total`        | gauge   | Number of DHT nodes, connected to                                |
| `qbittorrent_downloaded_bytes`       | counter | Data downloaded since the server started, in bytes               |
| `qbittorrent_uploaded_bytes`         | counter | Data uploaded since the server started, in bytes                 |
| `qbittorrent_torrent_category_total` | gauge   | Number of torrents for each category and status.                 |

## Installation

### Arch Linux
* Download release package
* Extract the content: `tar -xvzf prometheus-qbittorrent-exporter.tar.gz`
* Build and install: `makepkg -si`
* (Optional) Enable bundled systemd service to autostart exporter service on boot: `sudo systemctl enable --now prometheus-qbittorrent-exporter.service`

## Configuration

This exporter is primarily configured via CLI arguments:
```
--exporter-port <EXPORTER_PORT>                [default: 7071]
--qbittorrent-endpoint <QBITTORRENT_ENDPOINT>  [default: http://localhost:8080/]
--qbittorrent-username <QBITTORRENT_USERNAME>  [default: admin]
--qbittorrent-password <QBITTORRENT_PASSWORD>  [default: adminadmin]
```

In standard installation any of the required CLI arguments can be passed via `/etc/conf.d/prometheus-qbittorrent-exporter` environment file
(see `prometheus-qbittorrent-exporter.service` systemd service for more details).

Additionally, exporter log level can be configured via `RUST_LOG` environment variable (see [env_logger](https://docs.rs/env_logger/latest/env_logger/) documentation for more info).

## Similar projects
This project is heavily inspired by https://github.com/esanchezm/prometheus-qbittorrent-exporter.

Also, it would not possible without next libraries:
* https://github.com/George-Miao/qbit
* https://github.com/prometheus/client_rust
* many more...
