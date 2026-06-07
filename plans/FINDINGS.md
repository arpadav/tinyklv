# Findings

Current open items and benchmark notes. This file intentionally omits resolved
codegen/refactor findings so future agents do not re-walk completed work.

## To investigate

- **Wire size vs protobuf.** tinyklv streams are sometimes smaller than protobuf
  and sometimes larger. The known sparse-data issue is proto3 default-field
  omission: protobuf omits scalar default values and restores them on decode,
  while KLV currently emits every encoded field. Investigate whether optional
  field omission is a worthwhile KLV format option, and document the tradeoff
  against explicit presence on the wire.
- **Sentinel search implementation.** Re-benchmark the current memchr-backed
  sentinel search and the LazyLock cost. Confirm whether the optimization is
  still required on current Rust/memchr, and whether sentinel-free containers
  should have a cleaner trait shape than an optional/no-op sentinel path.
- **Large-payload encode sizing.** The current encoder uses direct fixed-width
  writes, fixed-length backpatching, scratch staging for variable-width length
  prefixes, and compile-time reserve hints. Revisit a prost-style
  `encoded_len()` pass only if profiling large payloads shows capacity growth
  dominating.

## General todos

- Add a README comparison table for tinyklv, manual code, and the benchmarked
  KLV/TLV crates. Use check/X/- style feature rows such as partial packet
  recovery, optional fields, optimized streaming, nested KLV support, unknown-key
  handling, and benchmark position. Verify every claim against source.
- Move benchmark methodology details into a `benches/README.md` so README can
  stay focused on the summary table and speed results.
- Re-check Kani status and either fix the proofs or document the current blocker.
- Reintroduce hard guardrails for `expect`, `panic`, `unwrap`, and raw slice
  indexing inside `tinyklv`, `tinyklv-impl`, and generated proc-macro tokens.
  If lints cannot catch generated code, add ripgrep-based checks.
- For future performance work, compare emitted LLVM/disassembly for the hot
  encode/decode paths before changing the codegen model.

## Tried and rejected / current approach

- **`Vec<T>::decode_value` pre-sizes** with a memory-bounded `with_capacity`
  hint. Starting from `Vec::new()` was ~2.7x slower on small runs. Do not revert
  without new benchmark data. (`src/traits/dec/mod.rs`)
- **The `Vec<T>` per-element checkpoint/eof loop stays.** The reset preserves the
  zero-consumed-rewind contract; the measured cost was missing capacity, not the
  loop. Removing it is high risk with little expected reward.
  (`src/traits/dec/mod.rs`)
- **No `*_from_slice` fixed-width decoder family.** A probe showed simple decode
  already ties manual code because LLVM elides inner `take(N)` checks after the
  outer length-bounded slice. (`src/codecs/binary/dec.rs`)
- **Encode does not chase prost on compound/rich by changing the wire format.**
  The remaining gap is mostly varint density vs fixed-width KLV bytes.
- **Streaming fresh-mode decodes complete framed bodies with one-shot
  `decode_value`.** Partial/resume machinery is for cross-buffer packets. The
  residual streamed gap to manual is sentinel seeking plus buffer bookkeeping.
  (`src/decoder/iter.rs`)
- **Bench `Reading::pack` allocates a temporary `Vec`.** Left as-is because the
  manual approach calls the same function. Removing it from both is optional
  benchmark hygiene only. (`benches/suite/records/shared.rs`)

## Benchmark / cross-crate notes

- **Field ordering is not a differentiator.** Every benchmarked decoder reads a
  key/tag and matches in a loop, so all are order-tolerant and skip unknown
  fields.
- **proto3 zero-field omission matters on sparse data.** The benchmark fills
  fields with non-default values, so it does not show protobuf's sparse-message
  size edge. This is the main known reason tinyklv streams may be larger on real
  sparse records.
- **KLV wins nested decode.** A KLV sub-packet decodes in place; protobuf embedded
  messages are length-delimited sub-records wrapped in generated optional fields.
- **Manual is the speed floor, tlv_parser is the slow ceiling.** tinyklv sits
  close to manual while preserving derive-based maintainability.
- **Reading the bench numbers:** encode timings include domain-value-to-message
  conversion for protobuf and struct-to-bytes conversion for tinyklv. Protobuf
  native timestamp/duration support differs by crate, so rich-record encode is
  not a same-wire comparison.
