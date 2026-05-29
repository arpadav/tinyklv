# tinyklv DECODE status

**Goal (achieved).** Close tinyklv's decode gap to `prost`. As of the current `bench.csv`, tinyklv
**decode beats `prost` on every record** and beats every other protobuf/KLV crate; only the
hand-written `manual` baseline is faster. Decode is in good shape — this file now tracks what was
done, what is deliberately left, and the one deferred opportunity.

This is decode only. Encode is tracked separately (`TKLV_ENCODER_IMPROVEMENT_PLAN.md`).

## Where decode stands (median ns/call, lower = faster; bold = fastest non-`manual`)

| record · decode | tinyklv | manual | serde_klv | tlv_parser | prost | quick_pb | rust_pb | micropb |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| simple · value   | **19.3** | 10.1 | 74.8  | 525.2  | 23.3  | 26.9  | 29.4  | 34.6  |
| compound · value | **51.3** | 19.5 | 110.4 | 739.0  | 122.5 | 122.5 | 152.2 | 133.2 |
| rich · value     | **43.0** | 31.4 | 153.9 | 931.2  | 102.9 | 78.4  | 118.9 | 87.6  |
| simple · frame   | **21.2** | 10.6 | 75.5  | 527.9  | 25.5  | 29.8  | 30.6  | 35.8  |
| compound · frame | **59.8** | 19.6 | 110.7 | 745.9  | 123.5 | 128.6 | 157.7 | 133.5 |
| rich · frame     | **51.7** | 32.0 | 146.8 | 923.1  | 101.1 | 79.2  | 119.8 | 88.0  |

tinyklv now leads `prost` on `simple` (it used to trail it). Framed overhead is small (~+2–9ns),
not the ~+50ns earlier analysis estimated — the short-sentinel seek is already inlined (below).

## Implemented (closed)

- **Winnow `be_*`/`le_*` → `take(N)` + `from_*_bytes`.** The dominant former tax — winnow's
  runtime-bounded shift/add byte loop — is gone. `src/codecs/binary/dec.rs` now does one sized read
  + one load + bswap (the `from_be_bytes(try_into)` shape `manual` uses). This single change is what
  moved `simple` decode below `prost`. The `*_lengthed` variants pad/truncate over a local zeroed
  array (no per-call const clone) and use `if`, not `match` on a bool.
- **Unchecked byte-array conversion for the integer leaves.** A microbench (`benches/int_decode.rs`,
  1000 samples) showed `*bytes.as_ptr().cast::<[u8; N]>()` beats the checked `try_from(..).expect(..)`
  on every integer width (non-overlapping CIs); it *regressed* `f32` and was a wash on `f64`. So the
  integer `be_*`/`le_*` ship the unchecked cast (via the debug-asserted `as_array_unchecked` helper,
  one `SAFETY` note) and the float decoders keep the checked form. Per-decode gain is ~0.01ns — small,
  but free and proven. `int_decode.rs` remains as the regression guard.
- **`#[inline]` on the generated `decode_value` and `Partial::finalize`.** Helps non-LTO downstream
  consumers; the bench builds with `lto=true, codegen-units=1` so it does not move these numbers.
- **Inlined short-sentinel seek.** Sentinels ≤ 4 bytes (the bench's 2-byte `b"\x47\x48"`) use an
  inline `memchr` first-byte scan; only longer needles use the cached `LazyLock<memmem::Finder>`.
  This is why framed overhead is now small.

## Still open (optional, measure first)

- **Bounds-check elision on fixed-width fields.** For a `var = false` field the outer loop already
  proved `eof_offset() >= len` and `subinput` is exactly `len` bytes, yet the leaf re-checks via its
  own `take(width)`. Passing the already-sliced `subinput` straight to `from_be_bytes` for
  statically-fixed-width fields would drop one branch per field. Small; LLVM may already fuse the
  adjacent checks. Only sound for fixed (`var = false`) codecs — never for `*_lengthed` (truncation/
  pad semantics). Needs an isolating micro-bench before investing; below the bar so far.

## Deferred — future opportunity (revisit)

- **In-place `decode_value` (drop the `Partial` round-trip) to chase `manual`.** Today the one-shot
  `decode_value` goes `decode_partial` → fill an `Option<T>`-per-field accumulator → `finalize`
  (a second pass). prost mutates `self` in place with no intermediate `Option`s and no finalize
  pass. A separate generated non-resumable `decode_value` that writes directly into locals seeded to
  their defaults (keeping the `Partial` path intact for streaming) would remove the per-field `.or()`
  merge + the second-pass match — prost's `merge_field` shape.

  **Why deferred:** decode already beats `prost`; the only faster decoder is hand-written `manual`,
  and this chases a few ns toward that floor at real cost. It touches derive output and must not
  disturb the streaming `Decoder` (`resume_partial` + `Packet::NeedMore` carrying the partial across
  `feed` boundaries). The regression danger is the rows where tinyklv already *wins* (compound/rich):
  the struct-literal-at-end with required-field guards must produce identical results for the nested
  records. If revisited, gate behind the full test suite and re-bench compound/rich for no
  regression; ship the low-risk variant (infallible `finalize` for all-default structs) first.

## Will not do

- **`Vec<T>` per-element checkpoint/eof loop** (`src/traits/dec/mod.rs`). The checkpoint/reset is
  load-bearing for the zero-consumed-rewind contract surrounding parsers rely on, and this is the
  path where tinyklv already beats `prost` on nested. High risk, ~zero reward. Leave it.

## How close to `manual` can decode get

`manual` is raw slice indexing with zero parser framework — the floor by construction. tinyklv's
residual gap to it is the resumable-decode machinery (checkpoints, `Packet`, the `Partial`
round-trip) and, on `rich`, native-type validation (`char::try_from`, `NonZeroU32::new`, date/time
range checks in `src/traits/native.rs`) that decodes into real Rust types — the feature, not a
defect. Matching `prost` while staying resumable and native-typed is the win; matching `manual`
would mean giving those up.
