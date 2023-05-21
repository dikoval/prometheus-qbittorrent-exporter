## test : Run project tests
test: Cargo.toml $(wildcard src/*)
	cargo test

## build : Build project executable (in debug mode)
build: target/debug/prometheus-qbittorrent-exporter
target/debug/prometheus-qbittorrent-exporter: Cargo.toml $(wildcard src/*)
	cargo build

## release : Build project executable (in release mode)
release: target/release/prometheus-qbittorrent-exporter
target/release/prometheus-qbittorrent-exporter: Cargo.toml $(wildcard src/*)
	cargo build --release

## package : Package project executable and other derivatives into archive
package: prometheus-qbittorrent-exporter.tar.gz
prometheus-qbittorrent-exporter.tar.gz: release $(wildcard dist/*)
	tar --create --gzip --verbose --file prometheus-qbittorrent-exporter.tar.gz \
		--directory dist/ $(shell ls dist) \
		--directory ../target/release/ prometheus-qbittorrent-exporter

## clean : Clean project artifacts
clean:
	rm -rf target/ prometheus-qbittorrent-exporter.tar.gz

## help : Print this help
help:
	@grep -E "^##" Makefile | cut --characters 4-

.PHONY: clean help
