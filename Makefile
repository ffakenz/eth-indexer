# Default target when running `make`
.DEFAULT_GOAL := help

# Variables
CARGO := cargo

.PHONY: help fmt fmt-check lint build test test-all clean

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

fmt: ## Format all Rust code
	$(CARGO) fmt --all

fmt-check: ## Check formatting (no changes)
	$(CARGO) fmt --all -- --check

lint: ## Run Clippy linter
	$(CARGO) clippy --all-targets --all-features -- -D warnings

build: ## Build the whole workspace
	$(CARGO) build --workspace

test: ## Run all tests (ignore docs)
	$(CARGO) test --workspace --tests

test-all: ## Run all tests
	$(CARGO) test --workspace

clean: ## Clean target directory
	$(CARGO) clean

check: ## Run fmt-check, lint, and tests
	@$(MAKE) fmt-check
	@$(MAKE) lint
	@$(MAKE) test-all

run: ## Run the CLI app
	cargo run --package cli -- $(filter-out $@,$(MAKECMDGOALS))

# --- DEMO (devnet) ---
.PHONY: demo demo.setup demo.play demo.engine

demo: ## Show available demo commands
	@echo "Available demo commands:"
	@echo "  make demo.setup     Deploy ERC20, mint tokens to ALICE, prepare .dev-env"
	@echo "  make demo.play      Run transfers between ALICE and BOB"
	@echo "  make demo.engine    Start the engine, watching demo contract"

demo.setup: ## Deploy ERC20 contract, mint tokens to ALICE, prepare .dev-env
	./resources/tests/demo/setup.sh

demo.play: ## Run transfers between ALICE and BOB
	./resources/tests/demo/play.sh

demo.engine: ## Start the engine to watch demo contract
	@set -a; \
	. resources/tests/demo/.env; \
	. resources/tests/demo/.dev-env; \
	set +a; \
	cargo run --package cli -- \
		--rpc-url "$$RPC_URL" \
		--db-url "sqlite::memory:" \
		--signer-pk "$$PK_ALICE" \
		--addresses "$$CONTRACT_ADDR" \
		--event transfer \
		--from-block "$$BLOCK_NBR" \
		--checkpoint-interval 10 \
		--poll-interval 100
