#!/usr/bin/env bash
EXAMPLES_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
shopt -s globstar
for f in "$EXAMPLES_DIR"/**/*.rs; do
    example="${f##*/}"       # strip path
    example="${example%.rs}" # strip .rs
    # the features this example's `required-features` names in Cargo.toml, if any
    features="$(awk -v name="\"$example\"" '
        $1 == "name" { found = ($3 == name) }
        found && $1 == "required-features" { gsub(/required-features|[][" =]/, ""); print; exit }
    ' "$EXAMPLES_DIR/../Cargo.toml")"
    cargo run ${features:+--features "$features"} --example "$example"
done
