# justfile — SDDK local CI aggregator
# Authority: AGENTS.md §5 (native commands remain authoritative when `just` is absent)

set shell := ["/bin/bash", "-euo", "pipefail", "-c"]

# Run all canonical local gates per AGENTS.md §5 + shell/python/js contract tests
ci:
	cargo fmt --all -- --check
	cargo build --release -p sddk-cli
	cargo test --workspace
	cargo clippy --workspace --all-targets -- -D errors
	@echo "=== sddk lint (local gate) ==="
	cargo run --locked -q -p sddk-cli -- lint --root . --format json
	@echo "=== ShellCheck (local gate) ==="
	@if command -v shellcheck >/dev/null 2>&1; then \
		for f in tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh; do \
			if [ -f "$f" ]; then shellcheck "$f" || exit 1; fi; \
		done; \
		echo "ShellCheck passed"; \
	else \
		echo "ERROR: shellcheck not installed — gate requires shellcheck (sudo apt install shellcheck)"; \
		exit 1; \
	fi
	@echo "=== Ruff (no Python scope — scripts/ contains only shell scripts) ==="
	@echo "Ruff has no scope: scripts/ contains only shell scripts (.sh), no Python files"
	@echo "=== Shell contract tests ==="
	for f in tests/test_*.sh; do bash "$f" || exit 1; done
	@echo "=== Node.js contract test ==="
	node tests/test_evidence_capture_contract.js
	@echo "=== Python contract tests ==="
	python3 tests/test_golden_dataset_contract.py
	python3 tests/test_workflow_contract.py
	@echo "=== all gates passed ==="
