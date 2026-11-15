.PHONY: docs-check fmt fmt-check check test clippy assets-check ci run

docs-check:
	markdownlint-cli2 "**/*.md"

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo check --workspace --all-targets --locked

test:
	cargo test --workspace --locked

clippy:
	cargo clippy --workspace --all-targets --locked -- -D warnings

assets-check:
	python3 -B -m unittest discover -s tools -p 'test_*.py' -v

ci: docs-check fmt-check assets-check check test clippy

run:
	cargo run --locked -p meridian-desktop
