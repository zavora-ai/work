.DEFAULT_GOAL := help
CORE := core
SHELL_DIR := shell

.PHONY: help check test lint fmt fmt-check clippy guardrails run clean hooks

help: ## Show this help
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-14s %s\n", $$1, $$2}'

check: ## Type-check the Core workspace
	cd $(CORE) && cargo check --workspace --all-targets

test: core-test shell-test ## Run every test

core-test: ## Run the Core tests
	cd $(CORE) && cargo test --workspace

shell-test: ## Run the Shell tests, including the handshake against the real Core
# Built with the engine, because that is the Core the app runs. Building it without leaves the same
# path holding a binary that cannot think, edit or speak — and the app says so in the User's words,
# which reads as a broken product rather than a build that was never asked for the engine.
	cd $(CORE) && cargo build -q -p studio-core --features adk
	cd $(SHELL_DIR) && npm run typecheck && npm test

lint: ## Run the vocabulary guardrail (Requirement 1.2)
	cd $(CORE) && cargo run -q --bin vocab-lint

adk-check: ## Verify the ADK-Rust adapter and all three specialists (needs sibling checkouts)
	cd $(CORE) && cargo clippy -p studio-runner --features adk --all-targets -- -D warnings
	cd $(CORE) && cargo test -p studio-runner --features adk
	cd $(CORE) && cargo clippy -p studio-core --features adk --all-targets -- -D warnings
	cd $(CORE) && cargo test -p studio-core --features adk

sheet-server: ## Build the spreadsheet capability server the integration test needs
	cd ../mcp-servers/worksheet-mcp && cargo build --bin excel-mcp-server

guardrails: lint ## Every build-failing guardrail
	cd $(CORE) && cargo test -p studio-lint -p studio-api

# The crates this repository owns, asked of the workspace rather than written down. `--no-deps`
# reports workspace members only, so a crate added tomorrow is formatted without anyone
# remembering to add it here — and a list maintained by hand is how a new crate quietly stops
# being checked.
OURS := $(shell cd $(CORE) && cargo metadata --no-deps --format-version 1 \
	| python3 -c "import json,sys; print(' '.join('-p ' + p['name'] for p in json.load(sys.stdin)['packages']))")

# Our crates only. `--all` reaches through the path dependency on adk-model into the sibling
# ADK-Rust checkout, so it would both reformat someone else's repository and fail our build for
# their unformatted work in progress. A guardrail that fires on another project's source teaches
# people to ignore it.
fmt: ## Format
	cd $(CORE) && cargo fmt $(OURS)

fmt-check: ## Verify formatting
	cd $(CORE) && cargo fmt $(OURS) -- --check

clippy: ## Lint Rust
	cd $(CORE) && cargo clippy --workspace --all-targets -- -D warnings

run: ## Run the Core
	cd $(CORE) && cargo run --bin studio-core

ci: fmt-check clippy test guardrails ## Everything CI runs

shell-build: ## Build the Shell
	cd $(SHELL_DIR) && npm run build

hooks: ## Install the pre-commit hook
	@mkdir -p .git/hooks
	@printf '#!/bin/sh\nexec make guardrails\n' > .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "pre-commit hook installed: make guardrails"

clean:
	cd $(CORE) && cargo clean
	rm -rf $(SHELL_DIR)/dist
