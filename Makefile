.PHONY: build test fmt lint clean help

help:
	@echo "StarInvoice Development Commands"
	@echo "================================"
	@echo "make build    - Compile contract to wasm32-unknown-unknown --release and optimize WASM"
	@echo "make test     - Run all tests"
	@echo "make fmt      - Format code with cargo fmt"
	@echo "make lint     - Run cargo clippy with strict warnings"
	@echo "make clean    - Remove build artifacts"

build:
	cargo build --target wasm32-unknown-unknown --release
	soroban contract optimize --wasm target/wasm32-unknown-unknown/release/star_invoice.wasm

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean
