#!/usr/bin/env bash

set -e

echo "🦑 Installing absolute-squid..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust and Cargo from https://rustup.rs/"
    exit 1
fi

# Install from local path
cargo install --path .

echo ""
echo "✨ Installation complete! You can now run 'absolute-squid' from anywhere."
echo "Make sure ~/.cargo/bin is in your PATH."
