.PHONY: docs-check

docs-check:
	markdownlint-cli2 "**/*.md"
