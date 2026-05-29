# List all available commands
default:
    @just --list

# List all available commands
help:
    @just --list

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

# Run a specific SDK example (e.g. `just example hello_world`, `just example agent_server`)
example name:
    #!/usr/bin/env bash
    set -euo pipefail
    # Normalize dashes to underscores
    name_clean=$(echo "{{name}}" | tr '-' '_')
    case "$name_clean" in
        "agent_server")
            echo "Starting agent_server sidecar..."
            cargo run --manifest-path examples/agent_server/Cargo.toml
            ;;
        "leptos_axum")
            echo "Starting leptos_axum example..."
            cd examples/leptos_axum && cargo leptos serve
            ;;
        "leptos_ssr_axum")
            echo "Starting leptos_ssr_axum example..."
            cd examples/leptos_ssr_axum && spin build --up
            ;;
        *)
            if [ -f "examples/${name_clean}.rs" ]; then
                cargo run --example "${name_clean}"
            else
                echo "Error: Example '${name_clean}' not found."
                echo "Available directory examples: agent_server, leptos_axum, leptos_ssr_axum"
                echo "Available file examples: hello_world, custom_tools, persistence, policies, streaming, structured_output, subagents"
                exit 1
            fi
            ;;
    esac


