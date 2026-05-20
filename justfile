# Run formatting, clippy lint checks, and unit/integration tests
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets --all-features

# Download and install the localharness binary
install:
    ./scripts/install_harness.sh

# Bump version and tag. By default, auto-bumps patch version. Override by passing version (e.g., just version 0.1.1).
version new_version="":
    @rustc scripts/bump-version.rs -o scripts/bump-version
    @scripts/bump-version "{{new_version}}"
    @rm scripts/bump-version

# Publish the SDK manually to crates.io (runs check first)
publish: check
    cargo publish

# Start localharness on the host
start-harness:
    cargo run --bin start-harness

