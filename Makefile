.PHONY: build build-musl build-alpine-docker test help

TARGET := aarch64-unknown-linux-musl
FRY_BIN := target/$(TARGET)/release/fry

build:
	cargo build --release

build-musl:
	@command -v aarch64-linux-musl-gcc >/dev/null || { \
	  echo "No aarch64-linux-musl-gcc; try: make build-alpine-docker"; \
	  exit 1; \
	}
	rustup target add $(TARGET)
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc \
		cargo build --release --target $(TARGET)
	@file "$(FRY_BIN)" || true
	@echo "Built $(FRY_BIN)"

build-alpine-docker:
	@command -v docker >/dev/null || { echo "docker required"; exit 1; }
	docker run --platform linux/arm64 --rm \
	  -v "$(CURDIR)":/src -w /src \
	  rust:alpine cargo build --release
	@file target/release/fry || true
	@echo "Built target/release/fry (static musl, Linux arm64)"

test:
	cargo test

help:
	@echo "Targets: build, build-musl, build-alpine-docker, test"