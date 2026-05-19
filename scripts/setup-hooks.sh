#!/bin/bash
set -e

# Get repository root directory
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "=== Setting up Git Hooks ==="

# Make hooks executable
chmod +x .githooks/pre-commit

# Configure git to use the custom hooks directory
git config core.hooksPath .githooks

echo "Git hooks configured successfully!"
