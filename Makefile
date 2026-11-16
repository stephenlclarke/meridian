.PHONY: docs-check fmt fmt-check check test clippy assets-check ci run preview-plan preview-release

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

# Preview packages and notes are prepared by the existing platform owners.
preview-plan:
	python3 -B tools/preview_release.py --commit "$(PREVIEW_COMMIT)" --plan

preview-release:
	python3 -B tools/preview_release.py --commit "$(PREVIEW_COMMIT)" --macos "$(PREVIEW_MACOS)" --windows "$(PREVIEW_WINDOWS)" --notes "$(PREVIEW_NOTES)" --receipt "$(PREVIEW_RECEIPT)"
