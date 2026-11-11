# Run Rust formatter in check mode
fmt:
	cargo fmt -- --check

# Run Clippy linter with warnings as errors
clippy:
	cargo clippy -- -D warnings

# Run the test suite
test:
	cargo test

# Install locally
lean:
	cargo install --path .

# Preview package contents without cleaning
package:
	cargo package --allow-dirty --list

# Run all checks: formatting, linting, and tests
check: fmt clippy test

# Preview a release (dry run)
release-dry-run level="patch":
	cargo release {{level}}

# Execute a release
release level="patch":
	cargo release {{level}} --execute
