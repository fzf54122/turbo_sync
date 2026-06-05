SHELL := /bin/sh

BIN ?= cargo run -p turbosync-cli --
SELFTEST_DIR ?= /tmp/turbosync-selftest
SELFTEST_CONFIG ?= $(SELFTEST_DIR)/local/config.toml
SELFTEST_DB ?= $(SELFTEST_DIR)/local/turbosync.db

.PHONY: help fmt fmt-fix clippy test check build build-gui release release-gui gui clean init agent dashboard status logs selftest-init selftest-agent selftest-dashboard

help:
	@printf '%s\n' 'TurboSync targets:'
	@printf '%s\n' '  make check              Run fmt check, clippy, and tests'
	@printf '%s\n' '  make fmt                Check formatting'
	@printf '%s\n' '  make fmt-fix            Format the workspace'
	@printf '%s\n' '  make clippy             Run clippy with warnings denied'
	@printf '%s\n' '  make test               Run all workspace tests'
	@printf '%s\n' '  make build              Build the workspace'
	@printf '%s\n' '  make build-gui          Build the desktop GUI'
	@printf '%s\n' '  make release            Build release binaries'
	@printf '%s\n' '  make release-gui        Build the desktop GUI release binary'
	@printf '%s\n' '  make gui                Run the desktop GUI'
	@printf '%s\n' '  make init               Run tsync init'
	@printf '%s\n' '  make agent              Run the local agent'
	@printf '%s\n' '  make dashboard          Open the TUI dashboard'
	@printf '%s\n' '  make status             Show agent status'
	@printf '%s\n' '  make logs               Show recent sync logs'
	@printf '%s\n' '  make selftest-init      Init an isolated /tmp self-test config'
	@printf '%s\n' '  make selftest-agent     Run agent with the self-test config'
	@printf '%s\n' '  make selftest-dashboard Open dashboard with the self-test config'

fmt:
	cargo fmt --all --check

fmt-fix:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

check: fmt clippy test

build:
	cargo build --workspace

build-gui:
	cargo build --manifest-path crates/turbosync-gui/Cargo.toml

release:
	cargo build --release --workspace

release-gui:
	cargo build --release --manifest-path crates/turbosync-gui/Cargo.toml

gui:
	cargo build -p turbosync-agent -p turbosync-cli
	cargo run --manifest-path crates/turbosync-gui/Cargo.toml

clean:
	cargo clean

init:
	$(BIN) init

agent:
	$(BIN) agent run

dashboard:
	$(BIN) dashboard

status:
	$(BIN) status

logs:
	$(BIN) logs --limit 20

selftest-init:
	mkdir -p '$(SELFTEST_DIR)/local'
	TURBOSYNC_CONFIG='$(SELFTEST_CONFIG)' TURBOSYNC_DB='$(SELFTEST_DB)' $(BIN) init

selftest-agent:
	TURBOSYNC_CONFIG='$(SELFTEST_CONFIG)' TURBOSYNC_DB='$(SELFTEST_DB)' $(BIN) agent run

selftest-dashboard:
	TURBOSYNC_CONFIG='$(SELFTEST_CONFIG)' TURBOSYNC_DB='$(SELFTEST_DB)' $(BIN) dashboard
