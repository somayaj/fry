.PHONY: build build-musl build-alpine-docker test deploy-nanovm help

TARGET := aarch64-unknown-linux-musl
FRY_BIN := target/$(TARGET)/release/fry
NANOVM_ROOT ?= $(abspath ../nanovm)
FRY_DATA_DIR ?= /var/lib/nanovm/fry

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
	@echo "Built target/release/fry (Alpine musl via Docker)"

test:
	cargo test

deploy-nanovm: build-musl
	@test -x "$(NANOVM_ROOT)/scripts/deploy-fry.sh" || { \
	  echo "Missing $(NANOVM_ROOT)/scripts/deploy-fry.sh"; exit 1; \
	}
	FRY_ROOT="$(CURDIR)" "$(NANOVM_ROOT)/scripts/deploy-fry.sh"

help:
	@echo "Targets: build, build-musl, test, deploy-nanovm (needs VM up)"
	@echo "  NANOVM_ROOT=$(NANOVM_ROOT)"