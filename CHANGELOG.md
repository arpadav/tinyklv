# Changelog

All notable changes to `tinyklv` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-05-28

### Added

- ASCII-text value codecs in `dec::string` / `enc::string`: decode and encode
  base-10 integers (`u8`..`u128`, `i8`..`i128`), floats (`f32`/`f64`), base-16
  integers (`hex_*`), and `alpha` / `digit` / `alphanumeric` run validators, all
  length-bounded and usable via `#[klv(dec = ..., enc = ...)]`.
- `enc::string::from_string_ascii` encoder, the encode counterpart to the
  existing `to_string_ascii` decoder.
- `BerOid::value(&self) -> T` accessor instead of direct field access.
- Benchmarks in `benches/` directory using `criterion`: benchmarking this crate
  against existing KLV Rust frameworks (including manual encode/decoding)

### Changed

- Significantly encoding - single allocation instead of double via `into_klv`.
- The `ascii` feature no longer depends on the external `ascii` crate; it now
  enables `winnow/ascii` instead.
- General style, doc comments, formatting, and linting improvements.
- `pastey` instead of `paste`

## [0.1.1] - 2026-04

## Removed

- Book and examples removed from crates.io to make more light-weight

## [0.1.0] - 2026-04

Initial release
