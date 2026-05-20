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
    @python3 scripts/bump-version.py "{{new_version}}"

# Publish the SDK manually to crates.io (runs check first)
publish: check
    cargo publish
