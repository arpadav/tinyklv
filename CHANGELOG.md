# Changelog

All notable changes to `tinyklv` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-rc.1] - 2026-05-29

Pre-release carrying breaking changes to the encode side and to decode-loop control.

### Added

- `#[klv(break_on = ...)]` container attribute and the `BreakType` enum
  (`Proceed` / `Skip` / `Done` / `Abort`) for decode-loop control. Accepts a key
  literal (a decoded key equal to it ends the loop) or a `fn(key, len) -> BreakType`.
- `size(..)` attribute model for field values and container defaults:
  `size(var)` selects length-parameterized decoders, `size(exact = N)` marks a
  fixed-width encoded value, and `size(hint = N)` supplies a soft reserve hint.

### Changed

- **Breaking:** `EncodeValue` and `EncodeFrame` no longer return owned bytes or carry
  an output type parameter. They now append into a caller-owned buffer:
  `encode_value(&self, out: &mut Vec<u8>)` / `encode_frame(&self, out: &mut Vec<u8>)`,
  so one `Vec<u8>` can be reused across many records and a value is encoded without a
  per-call heap allocation.
- Decoder internals reworked for throughput; the public `Decoder` / `DecoderIter` /
  `Packet` API is unchanged.
- Encoder internals reworked for throughput with direct fixed-width writes,
  fixed-length backpatching, and scratch staging only when the length prefix is
  variable-width.
- `break_on = <literal>` requires an integer-comparable key type; use the
  `break_on = <fn>` form for other key types.

### Removed

- **Breaking:** `IntoKlv` trait removed; key/length framing now happens inside
  `EncodeFrame::encode_frame`.
- **Breaking:** output-abstraction traits `EncodedOutput`, `TranscodableIterable`, and
  `HasElement` removed (encode targets `Vec<u8>` directly).
- **Breaking:** `codecs::binary::FixedLength` removed, along with its `decode` /
  `encode` / `decode_lengthed` / `encode_lengthed` helpers.
- **Breaking:** `BreakCondition` trait and `BreakConditionType` enum removed; use the
  `break_on` attribute and `BreakType` instead.
- Prelude no longer re-exports `BreakCondition`, `BreakConditionType`, `EncodedOutput`,
  or `IntoKlv`; it now re-exports `BreakType`.

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
