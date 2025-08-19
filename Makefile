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
	cargo run --package cli -- engine $(filter-out $@,$(MAKECMDGOALS))

query: ## Run the CLI query app
	cargo run --package cli -- select $(filter-out $@,$(MAKECMDGOALS))

# --- DEMO (devnet) ---
.PHONY: demo demo.setup demo.play demo.engine demo.query.checkpoint demo.query.transfer demo.test

demo: ## Show available demo commands
	@echo "Available demo commands:"
	@echo "  make demo.setup     			Deploy ERC20, mint tokens to ALICE, prepare .dev-env"
	@echo "  make demo.play      			Run transfers between ALICE and BOB"
	@echo "  make demo.engine    			Start the engine, watching demo contract"
	@echo "  make demo.query.checkpoint		Run engine query to select checkpoint outcomes"
	@echo "  make demo.query.transfer      	Run engine query to select transfer outcomes"
	@echo "  make demo.test      			Run setup, play, engine, play and query outcomes"

demo.setup: ## Deploy ERC20 contract, mint tokens to ALICE, prepare .dev-env
	./resources/tests/demo/setup.sh

demo.play: ## Run transfers between ALICE and BOB
	./resources/tests/demo/play.sh

demo.engine: ## Start the engine to watch demo contract
	@set -a; \
	. resources/tests/demo/.env; \
	. resources/tests/demo/.dev-env; \
	set +a; \
	cargo run --package cli -- engine \
		--rpc-url "$$RPC_URL" \
		--db-url "sqlite:$$DB_FILE" \
		--signer-pk "$$PK_ALICE" \
		--addresses "$$CONTRACT_ADDR" \
		--event transfer \
		--from-block "$$BLOCK_NBR" \
		--checkpoint-interval 3 \
		--poll-interval 100

demo.query.checkpoint: ## Run engine query to select checkpoint outcomes
	@set -a; \
	. resources/tests/demo/.env; \
	. resources/tests/demo/.dev-env; \
	set +a; \
	cargo run --package cli -- select \
		--db-url "sqlite:$$DB_FILE" \
		--entity checkpoint \
		--from-block "last"

demo.query.transfer: ## Run engine query to select transfer outcomes
	@set -a; \
	. resources/tests/demo/.env; \
	. resources/tests/demo/.dev-env; \
	set +a; \
	cargo run --package cli -- select \
		--db-url "sqlite:$$DB_FILE" \
		--entity transfer \
		--from-block "$$BLOCK_NBR"

demo.test: ## Run setup, engine, play and query outcomes
	./resources/tests/demo/test.sh
