#!/usr/bin/env bash
EXAMPLES_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
for f in "$EXAMPLES_DIR"/*.rs; do
    example="${f##*/}"       # strip path
    example="${example%.rs}" # strip .rs
    cargo run --example "$example"
done
