.PHONY: build release test lint fmt check clean link unlink run help

# Default target
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build debug binary
	cargo build

release: ## Build optimized release binary
	cargo build --release

test: ## Run all tests
	cargo test

lint: ## Run clippy lints
	cargo clippy --all-targets -- -D warnings

fmt: ## Format code
	cargo fmt

check: fmt lint test ## Format, lint, and test (CI-style)

clean: ## Remove build artifacts
	cargo clean

link: release ## Build release and symlink to /usr/local/bin/forge
	ln -sf $(CURDIR)/target/release/forge /usr/local/bin/forge
	@echo "Linked: /usr/local/bin/forge → $(CURDIR)/target/release/forge"

unlink: ## Remove the /usr/local/bin/forge symlink
	rm -f /usr/local/bin/forge
	@echo "Removed /usr/local/bin/forge"

run: ## Run forge (pass ARGS="..." for arguments, e.g. make run ARGS="notes k8s")
	cargo run --bin forge -- $(ARGS)

dry-run: ## Run forge in dry-run mode (pass ARGS="...")
	cargo run --bin forge -- --dry-run $(ARGS)
