.PHONY: source-check fmt-check

source-check:
	python3 -B tools/verify_sources.py

fmt-check:
	rustfmt --edition 2024 --check src/lib.rs tests/controller.rs tests/publication.rs tests/compatibility.rs tests/support/compat_cases.rs
