use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use futures_util::future;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::{Registry, Unit};
use qbit_rs::model::{Credential, GetTorrentListArg, Torrent};
use qbit_rs::model::ConnectionStatus::Connected;
use qbit_rs::model::State::Unknown;
use qbit_rs::Qbit;
use reqwest::Url;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct TorrentCategoryLabels {
    category: String,
    state: String
}

pub struct QBitMetrics {
    qbit_client: Qbit,

    // general status
    status_gauge: Gauge,
    dht_nodes_gauge: Gauge,

    // download/upload stats
    downloaded_bytes_counter: Counter,
    uploaded_bytes_counter: Counter,

    // torrent state/category stats
    torrent_category_gauge: Family<TorrentCategoryLabels, Gauge>
}

impl QBitMetrics {
    pub fn new(registry: &mut Registry, qtorrent_endpoint: String, username: String, password: String) -> Self {

        let qbit_client = Self::create_api_client(qtorrent_endpoint, username, password);

        // general status
        let status_gauge = Gauge::default();
        registry.register(
            "qbittorrent_status",
            "Current status (connected/not connected) of QBittorrent instance",
            status_gauge.clone()
        );
        let dht_nodes_gauge = Gauge::default();
        registry.register(
            "qbittorrent_dht_nodes_total",
            "Number of DHT nodes, connected to",
            dht_nodes_gauge.clone()
        );

        // download/upload stats - `_bytes_total` suffix will be added automatically by library :/
        let downloaded_bytes_counter = Counter::default();
        registry.register_with_unit(
            "qbittorrent_downloaded",
            "Data downloaded since the server started, in bytes",
            Unit::Bytes,
            downloaded_bytes_counter.clone()
        );
        let uploaded_bytes_counter = Counter::default();
        registry.register_with_unit(
            "qbittorrent_uploaded",
            "Data uploaded since the server started, in bytes",
            Unit::Bytes,
            uploaded_bytes_counter.clone()
        );

        // torrent state/category stats
        let torrent_category_gauge = Family::<TorrentCategoryLabels, Gauge>::default();
        registry.register(
            "qbittorrent_torrent_category_total",
            "Number of torrents for each category and status",
            torrent_category_gauge.clone()
        );

        return Self {
            qbit_client,
            status_gauge, dht_nodes_gauge,
            downloaded_bytes_counter, uploaded_bytes_counter,
            torrent_category_gauge
        };
    }

    fn create_api_client(qtorrent_endpoint: String, username: String, password: String) -> Qbit {
        let endpoint = Url::parse(qtorrent_endpoint.as_str())
            .expect(format!("Invalid QBittorrent URL provided: {}", qtorrent_endpoint).as_str());
        let credential = Credential::new(username, password);

        // disable connection pooling as it leads to "IncompleteMessage" errors
        // https://github.com/hyperium/hyper/issues/2136
        let client = reqwest::Client::builder()
            .tcp_keepalive(Duration::new(15, 0))
            .pool_max_idle_per_host(0)
            .build()
            .expect("Failed to build Reqwest HTTP client");

        return Qbit::new_with_client(endpoint, credential, client);
    }

    pub async fn update_metrics(&self) -> Result<(), Box<dyn Error>> {
        let result = future::try_join(
            self.report_status_metrics(),
            self.report_torrent_metrics()
        );

        // ignore result
        return result.await.map(|_| ());
    }

    async fn report_status_metrics(&self) -> Result<(), Box<dyn Error>> {
        let info = self.qbit_client.get_transfer_info().await?;

        if info.connection_status == Connected {
            self.status_gauge.set(1);
        } else {
            self.status_gauge.set(0);
        }

        self.dht_nodes_gauge.set(info.dht_nodes as i64);

        let download_inc = info.dl_info_data - self.downloaded_bytes_counter.get();
        self.downloaded_bytes_counter.inc_by(download_inc);

        let upload_inc = info.up_info_data - self.uploaded_bytes_counter.get();
        self.uploaded_bytes_counter.inc_by(upload_inc);

        Ok(())
    }

    async fn report_torrent_metrics(&self) -> Result<(), Box<dyn Error>> {
        let torrents = self.qbit_client.get_torrent_list(GetTorrentListArg::default()).await?;

        let mut stats: HashMap<TorrentCategoryLabels, i64> = HashMap::new();
        for torrent in torrents.iter() {
            let labels = self.extract_labels(torrent);
            let current_count = stats.get(&labels).unwrap_or(&0);
            let new_count = current_count + 1;
            stats.insert(labels, new_count);
        }

        // update gauge
        self.torrent_category_gauge.clear();
        for (labels, count) in stats {
            self.torrent_category_gauge.get_or_create(&labels).set(count);
        }

        Ok(())
    }

    fn extract_labels(&self, torrent: &Torrent) -> TorrentCategoryLabels {
        let category = torrent.category.clone().unwrap_or("<None>".to_string());
        let state = torrent.state.clone().unwrap_or(Unknown);
        let state = format!("{:?}", state);

        return TorrentCategoryLabels { category, state };
    }
}
