# tinyklv ENCODE improvement plan (ENCODE ONLY)

Goal: close tinyklv's **encode** gap to `prost`/`manual` - driven by allocation strategy, not wire
format - without changing the encoded bytes and without breaking the public `EncodeValue` API. This
is analysis + a prioritized, line-referenced plan; no source modified yet. It is the encode
counterpart to `TKLV_DECODER_IMPROVEMENT_PLAN.md` (which is decode-only and explicitly defers encode).

Baseline (old bench, `KLV_VS_PROTOBUF.md` rows 23/25/27 - refresh from the latest `simple/compound/
rich · encode · value` run):

| encode · value | tinyklv | manual | prost |
|---|---|---|---|
| simple (was flat)    | 75.3 | 67.1 | **34.2** |
| compound (was nested)| 144.7 | 129.2 | **85.5** |
| rich (was native)    | 224.6 | 161.3 | **115.5** |

`prost` wins every encode row; `manual` beats tinyklv everywhere. The cause is the same one
`KLV_VS_PROTOBUF.md:64-93` calls out: **tinyklv allocates a small owned `Vec` per field (and per
key, per length) and concatenates them**, whereas `prost` does one sized allocation + linear writes,
and `manual` writes `to_be_bytes()` stack arrays into a single `Vec` with no per-field heap.

---

## Where the allocations are (verified against source)

The derived encoder is structurally fine (monomorphized, single struct pass) but allocation-heavy:

1. **`impl/src/expand/encode_impl.rs:64-69`** - `encode_value` opens `let mut output = vec![]` with
   **no capacity hint**, so it reallocs as it grows.
2. **`impl/src/expand/encode_impl.rs:163-166` (optional) and `:171-174` (required)** - per field, the
   generated body is:
   ```rust
   let __value = #enc_tokens(#arg);          // owned Vec<u8>  (alloc #1)
   output.extend(#key_encoder(#key));        // owned Vec<u8>  (alloc #2)
   output.extend(#len_encoder(__value.len()));// owned Vec<u8> (alloc #3)
   output.extend(__value);                   // moves alloc #1's bytes, drops it
   ```
   -> **~3 heap allocations per field**, all immediately consumed by `extend`.
3. **`src/codecs/binary/enc.rs:25-63`** - the leaf encoders are the alloc source:
   `pub fn be_u64(input) -> Vec<u8> { input.to_be_bytes().to_vec() }`. `to_be_bytes()` is a stack
   array; `.to_vec()` is the gratuitous heap copy. Same shape in `ascii`, `ber/mod.rs:174,215`, and
   `string` encoders (all `-> Vec<u8>` / collect).
4. **`impl/src/expand/encode_impl.rs:51-58` + `src/traits/enc.rs:203-212`** - `encode_frame` =
   `self.encode_value()` (full value Vec) `.into_klv(sentinel, len_encoder)`, and `IntoKlv::into_klv`
   does `key.into_iter().chain(len).chain(self).collect()` - a **second** full allocation + element
   copy of the entire value. So framed encode pays the value cost twice.

`manual`'s win is the tell: `out.extend_from_slice(&field.to_be_bytes())` - no per-field heap. The
target is to make the derive emit that shape.

---

## Design decision (the `enc` fn contract) - OPEN, confirm before coding

The per-field value `Vec` (alloc #1) is inherent to the encoder contract `enc: fn(T) -> Vec<u8>`. To
remove it the encoder must *write into* a caller buffer. Three ways, in increasing disruption:

- **(A) Additive `encode_into` path (recommended).** Introduce a buffer-writing encoder surface and
  an opt-in derive flag, leaving the existing `-> Vec<u8>` encoders and default codegen untouched.
  Per the project rule "new proc-macro codegen behavior gets an opt-in flag; don't change defaults
  silently," this is the safe shape. Opt-in struct attr (name OPEN): `#[klv(encode_into)]`.
- **(B) Capacity + dedupe key/len only.** Keep `enc(arg) -> Vec`, but give `output` a capacity hint
  and write key/len via stack arrays. Kills allocs #2/#3 and the realloc churn, not #1. Small, no
  flag, no contract change. A cheap always-on first step.
- **(C) Replace the encoder contract** (`fn(T, &mut Vec<u8>)`). Largest win, but a breaking change to
  every `enc = ...` attribute and hand-written encoder. Not recommended.

Recommended: ship **(B)** as the always-on baseline, then **(A)** as the opt-in prost-parity path.

### K-L-V ordering problem (applies to A and C)

KLV writes key, then **length**, then value - but the length is the *encoded value length*, which a
streaming write doesn't know until the value is written. Three resolutions:

- **(A1) Scratch buffer (recommended, general).** One reused `scratch: Vec<u8>` for the whole record;
  per field: `scratch.clear(); value_encode_into(arg, &mut scratch); key_into(&mut out);
  len_into(scratch.len(), &mut out); out.extend_from_slice(&scratch);`. Allocations drop from ~3N to
  **one reused scratch + the output** (and `scratch` reaches steady-state capacity after the first
  field). Works for any length encoder (BER included).
- **(A2) Backpatch.** Write key, reserve length slot(s), write value directly into `out`, then patch
  the length in place. Zero scratch, but only sound for **fixed-width** length encoders (1-byte len
  is the common case); variable-width (BER) length-of-length makes the slot size unknown. Gate on a
  fixed-width-len marker if pursued.
- **(A3) Size pass (`encoded_len`).** prost's approach: a pure `encoded_len()` per field/leaf, one
  `Vec::with_capacity(total)`, then a single linear write with lengths known up front. Most code
  (every leaf needs an `encoded_len`), best result, no scratch. Heaviest lift.

Recommend **(A1)** for the opt-in path: general, modest code, removes the dominant churn. Note (A3)
as the eventual ceiling if profiling still shows capacity reallocs dominating on large payloads.

---

## STEP 1 (always-on, no flag) - capacity hint + stack-array key/len writes

- `encode_impl.rs:66` - `let mut output = vec![]` -> `Vec::with_capacity(N)` where N is a cheap
  compile-time lower bound (sum of fixed key+len widths; 0 is acceptable if unknowable). Removes
  early reallocs.
- `encode_impl.rs:164-165,172-173` - replace `output.extend(#key_encoder(#key))` /
  `output.extend(#len_encoder(__value.len()))` with writes that don't allocate when the encoder
  output is a stack array. Cleanest without touching the encoder contract: keep the calls but ensure
  the leaf key/len encoders return `impl AsRef<[u8]>` / arrays rather than `Vec` where the width is
  fixed - OR add `*_into` forms for key/len only (smaller blast radius than all value encoders).
- **Risk: LOW.** No wire change, no API break. Re-bench `*_encode_value`/`*_encode_frame`.

## STEP 2 (opt-in `#[klv(encode_into)]`) - buffer-writing value path (the prost-parity win)

- **Trait (`src/traits/enc.rs:135-143`).** Add a sibling method (keep `encode_value` as-is for
  back-compat):
  ```rust
  fn encode_into(&self, out: &mut O) { /* default: out.extend(self.encode_value()) */ }
  ```
  Default-implemented in terms of `encode_value` so existing manual impls keep working; the derive
  overrides it with the real buffered body.
- **Leaf encoders.** Add `*_into(input, out: &mut Vec<u8>)` to `src/codecs/binary/enc.rs:25-63`
  (`out.extend_from_slice(&input.to_be_bytes())`), and the analogous forms in `src/codecs/ascii/`,
  `src/codecs/ber/mod.rs:174,215`, `src/codecs/string/`. Keep the `-> Vec<u8>` originals (one becomes
  a thin wrapper over the other to avoid drift). **Audit step: enumerate every leaf encoder.**
- **Codegen (`encode_impl.rs:62-73, 93-179`).** When the opt-in flag is set, emit `encode_into`
  using the **(A1) scratch** pattern above; `encode_value` becomes
  `{ let mut o = Vec::with_capacity(..); self.encode_into(&mut o); o }`. The `fallback_enc` arm
  (`encode_impl.rs:124-130`) recurses into the child's `encode_into` instead of `encode_value`.
- **Frame (`encode_impl.rs:51-58`, `enc.rs:203-212`).** Emit `encode_frame` to write key + length +
  value into **one** buffer (sentinel and length written first, value appended via `encode_into`),
  replacing the `encode_value().into_klv(..)` double allocation. Keep `IntoKlv` for non-derive users.
- **Invariants:** byte-identical output to today (optional fields still omitted on `None`, last-field
  ordering preserved, length semantics unchanged). **Risk: HIGH (derive output)** - gate on Step 4.

## STEP 3 - `EncodeAs` interaction (no change expected, verify)

The sigil path (`encode_impl.rs:143-156`) already avoids clones via `EncodeAs::encode_as` (by-ref /
by-value, no heap). Confirm the `encode_into` rewrite preserves that - the arg shaping is orthogonal
to where the bytes land.

## STEP 4 - Tests / verification (the backstop)

- **Differential:** for arbitrary records, assert `encode_into`/buffered output is **byte-identical**
  to the current `encode_value`/`encode_frame` (a proptest comparing old vs new path). The bench's
  `checks::verify` round-trip is necessary but not sufficient (it only checks decode-equality).
- Custom structs/enums with: required + optional fields, `None` omission, nested (`fallback_enc`),
  `Vec<T>` runs, BER length, native types - per repo norm (no primitive-only tests).
- **Re-bench** `benches/scripts/gencharts.sh -f`; confirm `*_encode_value`/`*_encode_frame` drop toward
  manual/prost, and **decode tiers unaffected** (encode path is separate).

## Realistic targets vs prost (encode · value)

- simple 75 -> ~40-50 (manual is 67; prost 34): scratch path removes per-field churn; a leftover gap
  to prost is varint-vs-fixed-width wire density, not allocation - inherent to KLV, not a defect.
- compound 145 -> ~95-110 (toward manual 129 / prost 86).
- rich 225 -> ~150-170 (toward manual 161); residual is native-type *validation*-free encode plus the
  per-field write - already close to manual once allocation is gone.

## Open questions (confirm before coding)

1. Opt-in flag name: `#[klv(encode_into)]`? something else? (mirrors `default_all`/`deny_unknown_keys`.)
2. K-L-V ordering: scratch (A1, general) vs backpatch (A2, fixed-len only) vs size-pass (A3, ceiling)?
3. Should Step 1 (capacity + key/len) be always-on, or also behind the flag to keep defaults frozen?
4. Leaf-encoder surface: add `*_into` fns (parallel API) vs an `EncodeInto` trait on the leaf types?

## Post-implementation review gate (per the decode-plan convention)

Re-run on the `impl/` + `src/` git diff: the four `aav-idiomatic-rust-*` lenses,
`aav-semantic-architecture-reviewer`, `aav-rust-perf`, and the Code Reviewer; address findings.
