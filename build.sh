#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Building release binary..."
cargo build --release

BINARY="./target/release/tig-review"
echo "Executable ready: $BINARY"
