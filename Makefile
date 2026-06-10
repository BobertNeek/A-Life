.PHONY: setup build run fmt check clippy test wasm

setup:
	./scripts/setup.sh

build:
	./scripts/build.sh

run:
	./scripts/run.sh

fmt:
	cargo fmt --all -- --check

check:
	cargo check --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

wasm:
	cargo check -p alife --target wasm32-unknown-unknown
