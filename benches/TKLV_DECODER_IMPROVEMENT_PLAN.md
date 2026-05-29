# tinyklv DECODE improvement plan

Goal: close tinyklv's decode gap to `prost`/`manual`, especially on **flat** (tinyklv 72ns
vs prost 61ns vs manual 85ns), **without regressing nested (189ns, beats prost 234) or native
(281ns, beats prost is borderline; manual floor 200)**. Decode only. Encode is a separate
concern and is not touched here except where leaf codecs are shared.

This is analysis + a prioritized plan. No source was modified.

---

## Where the time actually goes (verified against source)

The tinyklv decode loop is structurally good: it is monomorphized (no `dyn`), single-pass, and
genuinely zero-copy on the value bytes (`winnow::token::take` yields a `&[u8]` subslice, no copy).
The gap to `manual`/`prost` on `flat` is **constant per-field tax**, not algorithmic. Three taxes,
in descending cost order:

### Tax A — winnow `be_*` builds ints with a byte-by-byte shift/add loop (the big one)

`src/codecs/binary/dec.rs` `be_u32` etc. forward to `winnow::binary::be_u32`
(`winnow-0.7.15/src/binary/mod.rs:200`), which calls `be_uint(input, 4)`
(`mod.rs:296`), whose core is `to_be_uint` (`mod.rs:317`):

```rust
let mut res = Uint::default();
for (_, byte) in number.iter_offsets().take(offset) {  // offset is a RUNTIME value = min(4, remaining)
    res = (res << 8) + byte.into();
}
```

`offset` comes from `input.offset_at(bound)`, which returns the *runtime* min of the requested
width and remaining bytes. Because the trip count is a runtime value, LLVM cannot reliably unroll
this into a single 4-byte load + `bswap`; it stays a loop of `shl`/`add`/bounds-checked iterator
steps per byte. `manual/flat.rs` instead does:

```rust
c = Some(u32::from_be_bytes(val.try_into().ok()?));   // one 4-byte load + bswap, one length check
```

`from_be_bytes` over a `[u8; 4]` is a single aligned-agnostic load and a `bswap` instruction.
This is the single largest contributor to the flat decode gap: 8 fields × (loop vs single load) is
exactly the "tax that grows with field count" the analysis in `KLV_VS_PROTOBUF.md` observed
(closest to manual on flat where fields are few... actually flat has the most primitive fields, and
it is where tinyklv trails manual). `simd` feature on winnow does not help here — `be_uint` is the
generic integer path, not a SIMD token scan.

There is also the `trace("be_u32", ...)` wrapper on every call (`mod.rs:205`). `trace` is a no-op
in release (compiles away), so it is not a cost — verified, not a finding.

### Tax B — the `Partial` (`Option<T>` accumulator) struct + `finalize` round-trip

prost's generated `merge_field` mutates `self` **in place** (`prost-derive` expands a
`match tag { N => decode directly into self.field }`); missing fields keep their zero default. No
intermediate struct, no per-field `Option`, no second pass.

tinyklv (`impl/src/expand/decode_impl/`) instead:
1. allocates a `XxxPartialPacket` with one `Option<T>` **per field** (`partial_gen.rs:36`),
2. each match arm writes `__acc.field = dec(&mut subinput).ok().or(__acc.field)`
   (`key_match_gen.rs:188`) — an `Option` write plus an `.or()` merge every field,
3. then `Partial::finalize` (`partial_gen.rs:225`) does a **second pass**: one
   `match self.field { Some(v) => v, None => return Err(..) }` per required field, then moves
   everything into the final struct literal.

So every flat decode pays: 8 `Option` stores + 8 `.or()` merges in the loop, then 8
`Some`/`None` unwraps + 8 moves at finalize. That is the structural difference from prost's
single in-place write. It is mostly stack traffic (the `Option<u64>` etc. are `Copy`, no heap),
so LLVM with LTO will collapse a lot of it, but the `.or(__acc.field)` merge and the second-pass
match are real branchy work prost simply does not do.

The `.ok().or(__acc.field)` merge is **last-wins** (correcting an earlier misread): since
`Some(new).or(prev) == Some(new)`, a duplicate key that decodes successfully overwrites the
earlier value, while one that fails to decode keeps the prior valid one. This matches prost's
last-wins-for-scalars. Here it only matters that the `.or()` adds a branch per field.

### Tax C — `take(len)` re-slices and the per-iteration key/len/eof bookkeeping

Each loop iteration (`decode_partial_gen.rs:156`) does, per field:
- `input.eof_offset() == 0` check,
- `input.checkpoint()` (cheap for `&[u8]`, just a pointer copy),
- key decode, then `__key_offset_before - input.eof_offset()` consumed-bytes math,
- len decode,
- `break_condition(key, len)` dispatch (a match),
- `input.eof_offset() < len` bounds check,
- `take(len)` producing `subinput`,
- the `match key` dispatch.

`manual` does the equivalent with raw index arithmetic (`j += 2; body.get(j..j+len)?`) and one
`match` — far fewer distinct operations. Most of tinyklv's extra steps exist to support
**streaming resumption** (`Packet::NeedMore`, checkpoint rewinds) which `manual` and the bench's
non-streaming `decode_value` entry point do not need. This is the price of the resumable design.

### Non-finding — missing `#[inline]` on generated `decode_value`

`gen_decode_value_impl` (`mod.rs:229`) emits `fn decode_value(...)` with **no** `#[inline]`,
while `decode_partial`, `decoder()`, and `try_from` all carry `#[inline(always)]`. In a normal
dependent crate this would block inlining across the crate boundary. **But** the bench builds with
`lto = true, codegen-units = 1` (root `Cargo.toml:115-117`), so LTO inlines it regardless. Adding
`#[inline]` is still correct hygiene (it helps non-LTO consumers), but do **not** expect it to move
the bench numbers. Classified below as a cheap correctness-of-intent fix, not a bench win.

---

## Prioritized changes

### 1. Replace winnow `be_*`/`le_*` fixed-width int decoders with direct `from_be_bytes` — EASY FIX, biggest win

**What.** In `src/codecs/binary/dec.rs`, rewrite the `wrap!` macro bodies so `be_u32` etc. do a
single sized read + `from_*_bytes` instead of delegating to `winnow::binary::be_u32`:

```rust
#[inline(always)]
pub fn be_u32(input: &mut &[u8]) -> winnow::Result<u32> {
    // one bounds check, one 4-byte load, one bswap
    let bytes = winnow::token::take(4usize).parse_next(input)?;   // &[u8] of len 4, no copy
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))             // try_into on a known-4 slice
}
```

`take(4)` already does exactly one length check and advances the cursor; `try_into::<[u8;4]>`
on a slice the parser guaranteed is length-4 compiles to a single load (the `unwrap` is provably
unreachable and elided). This keeps winnow integration (`&mut &[u8]`, `winnow::Result`,
`ContextError`) and the `stream = &[u8]` genericity at the call sites intact — the *signature is
unchanged*, only the body. If you want to preserve generality over `Stream`, gate the fast body to
`&[u8]` and keep the winnow path as the generic fallback (the codegen already defaults
`stream = &[u8]`, so the common case hits the fast path).

**Why slow now / how manual avoids it.** Tax A above: winnow's `to_be_uint`
(`winnow-0.7.15/src/binary/mod.rs:317`) is a runtime-bounded shift/add byte loop; `manual` uses
`from_be_bytes(try_into)` (`manual/flat.rs:73`), one load + bswap.

**Expected impact.** Flat decode is 8 fixed-width fields — this is the dominant per-field cost.
Estimate the largest single chunk of the 72→~62ns gap comes from here; plausibly lands flat decode
at/near prost (61) and at or below manual (85). Helps native (12 fields, several int-backed) and
nested too, since `be_f64`/`be_u32`/`be_i16` all share this path. **Does not regress** nested/native
— it only makes each leaf cheaper.

**Risk.** Low. Body-only change, public signatures untouched, winnow `Result`/error type
unchanged. The `*_lengthed` variants already use `from_be_bytes` on the pad path — only the
full-width fast path needs care; keep their `take(len)` + pad logic. Verify the `try_into().unwrap()`
optimizes to a bare load on the target (`cargo asm` / a criterion micro-bench on `be_u32`).
Note: `be_u8`/`i8` are single-byte — just `take(1)` + index, trivial.

---

### 2. Add a non-resumable fast `decode_value` path that skips the `Partial` round-trip — FULL REFACTOR (codegen), medium win, guard against nested/native regression

**What.** The current `decode_value` (`mod.rs:229`) goes
`decode_partial` → fill `Option` accumulator → `finalize` (second pass). For the **one-shot**
decode the bench's `decode()` uses (`benches/suite/approaches/tinyklv/flat.rs:18`,
`Telemetry::decode_value(&mut input).ok()`), the `Partial`/`NeedMore` machinery is pure overhead:
the caller never resumes.

Two implementation options:

- **(2a) In-place direct construction.** Generate, alongside the resumable path, a direct
  `decode_value` that declares plain locals seeded to their `default(...)` (mirroring the partial's
  `Default`), runs the same `match key` loop writing **directly** into those locals (drop the
  `Option` wrapper for fields that have a `default`, and the `.or()` merge), and builds the struct
  literal at the end with the required-field guards inline. This is prost's `merge_field` shape:
  one write per field, no second pass. This is essentially resurrecting the historical
  `gen_items_default` local-accumulator form (referenced in `partial_gen.rs:31` docs) but **only**
  for the non-streaming entry point, keeping the `Partial` struct for the streaming `Decoder`.

- **(2b) Cheaper finalize.** Keep the partial but make `finalize` not a full second pass for the
  common all-fields-have-defaults case — i.e. when no field is required, `finalize` is infallible
  and can be a plain move; skip the per-field `match Some/None`. Smaller win, much smaller change.

**Why slow now / how prost avoids it.** Tax B: prost mutates `self` in place with zero
intermediate `Option`s and no finalize pass (`prost-derive` `merge_field`). tinyklv pays
`Option` store + `.or()` per field in the loop **and** a second unwrap+move pass in `finalize`.

**Expected impact.** Flat: removes 8 `.or()` merges + the 8-field second pass. Modest (LLVM+LTO
already collapses the `Copy` `Option` traffic somewhat), estimate a few ns on flat, more on native
(12 fields). Nested benefits less (sub-packet + Vec dominate).

**Risk / regression guard.** HIGH effort, touches derive output. Must **not** disturb the
streaming `Decoder` path (`src/decoder/iter.rs` `next_resume` relies on `resume_partial` +
`Packet::NeedMore` carrying the partial across `feed` boundaries — see memory note). Keep
`resume_partial` exactly as is; add 2a as a *separate* generated method that `decode_value` calls
instead of `decode_partial`+`finalize`. Because nested/native currently *win*, regressions there
are the real danger: 2a's struct-literal-at-end with required guards must produce identical results
for `GpsCoord`/`Platform`/`NativeNested`. Gate behind tests before trusting bench deltas. If 2a is
too invasive, ship **2b** first (low risk, partial win).

---

### 3. `#[inline]` the generated `decode_value` — EASY FIX, hygiene not bench win

**What.** Add `#[inline]` to the `fn decode_value` emitted in `mod.rs:229` (and consider
`#[inline]` on `Partial::finalize` in `partial_gen.rs:322`, currently un-annotated).

**Why.** Consistency with the already-`#[inline(always)]` siblings, and it helps **non-LTO**
downstream consumers of the crate inline the decode entry point.

**Expected impact.** ~0 on this bench (LTO already inlines — see Non-finding above). Real-world
win for users who build without fat LTO. Do not attribute any bench movement to this.

**Risk.** None. Additive attribute.

---

### 4. Bounds-check elision on the value read path — EASY-to-MEDIUM, small win, needs measurement

**What.** After step 1, the per-field read is `take(len)` (one check in `resume_partial`,
`decode_partial_gen.rs:312`) followed by the leaf's own `take(width)` (a second check). For
fixed-width fields the outer loop already verified `eof_offset() >= len` (line 296), and `subinput`
is exactly `len` bytes — yet the leaf re-checks against `subinput`. That is two bounds checks where
`manual` has one (`body.get(j..j+len)?` then `try_into`). Consider, in the codegen for fixed-width
fields, passing the already-sliced `subinput` straight to `from_be_bytes` without a second `take`,
since `len` is known to equal the field width for these fields (the wire is fixed-width KLV).

**Why slow now / how manual avoids it.** `manual` slices once (`val = body.get(j..j+len)?`) then
`val.try_into()` — the `try_into` length check is the only second check and folds with the first
under LLVM. tinyklv's leaf does an independent `take` on `subinput`.

**Expected impact.** Small (one branch per field), and LLVM may already fuse adjacent checks on the
same slice. **Needs measurement** — write a criterion bench isolating one field decode before/after.
Do not over-invest; this is below steps 1–2 in priority.

**Risk.** Medium — only valid for fields whose KLV length is statically the type width. KLV
permits truncated/over-long values (that is why `*_lengthed` exists), so this optimization is only
sound for fields declared with the fixed (`var = false`) codec. Mis-applying it to a `*_lengthed`
field would break the truncation/pad semantics.

---

### 5. `Vec<T>` repeated-field decode: drop per-element checkpoint where the inner decoder is infallible-on-progress — MEDIUM, nested-only, do not regress

**What.** `src/traits/dec/mod.rs:80` `Vec<T>::decode_value` does per element:
`before = eof_offset(); cp = checkpoint(); if Ok(val) push else { if eof unchanged reset(cp); break }`.
For the bench's `sensors: Vec<Reading>`, `Reading::decode_value` (`records.rs:181`) reads exactly
5 bytes and only fails at EOF. The checkpoint + `eof_offset` per element is there to rewind a
zero-consumed failure. For `&[u8]` `checkpoint()`/`reset()` are pointer copies (cheap), and
`eof_offset()` is a length read (cheap) — so this is a **minor** cost, not a hot smell. The
`Vec::new()` then grows without `with_capacity` (unknown count), causing realloc churn for large
sensor runs (here 1–8 elements, so ≤3 reallocs — negligible).

**Why / how others compare.** This is the path where tinyklv already **beats** prost on nested
(189 vs 234), because a KLV sub-run is cheaper than prost's length-delimited+`Option`/`Box`
embedded message. So there is little to gain and real risk in touching it.

**Expected impact.** Near zero on the current bench (small Vecs). **Recommend leaving as-is** unless
a profiler on a large-`sensors` workload shows the checkpoint loop hot. If you do touch it,
the only defensible change is hoisting nothing (the realloc is data-dependent) — so this is
effectively a **no-op recommendation**: do not spend effort here, and explicitly do not refactor it
in a way that could regress the nested win.

**Risk.** HIGH relative to reward. The checkpoint/reset is load-bearing for the
zero-consumed-rewind contract that surrounding parsers rely on. Leave it.

---

### 6. Framed decode (`decode_frame`) overhead — EASY-to-MEDIUM, framed-rows only

**What.** Framed adds ~50ns for tinyklv (flat 72→125, nested 189→232, native 281→325) vs ~15–30ns
for others. The cost is in `seek_sentinel` (`sentinel_gen.rs:84`): `Finder::find(&input)` (memchr,
fast) **plus** a `len_decoder.parse_next` **plus** `take(packet_len)` to carve the body, then
`decode_frame` (`src/traits/dec/frame.rs:27`) runs `decode_value` on that sub-slice and on error
calls `e.add_context(...)`. The `LazyLock<Finder>` deref (`sentinel_gen.rs:77`) is one atomic-ish
check per call. The bench's sentinel is 2 bytes (`b"\x47\x48"`, `records.rs:46`); `memmem::Finder`
for a 2-byte needle is heavier setup than a hand `windows(2).position`, though the `Finder` is
cached in the `LazyLock`.

The ~50ns vs ~15–30ns gap is plausibly: (a) `LazyLock` deref + `Finder::find` dispatch overhead for
a tiny needle vs the competitors' inline `windows(2).position`, and (b) the extra
`len_decoder.parse_next` + `take` that re-validates the body before decode.

**Why / measurement.** This is **hypothesis from source, not profiled.** Recommend a criterion
bench splitting `seek_sentinel` from `decode_value` to attribute the 50ns. If `Finder` setup/deref
dominates for the 2-byte needle, consider a specialized inline 2-byte/short-needle seek path (a
`memchr` on the first sentinel byte + a compare), bypassing `LazyLock<Finder>` for needles ≤ a few
bytes.

**Expected impact.** Framed rows only; could halve the framing tax (~25ns) if `Finder` overhead is
the cause. **Needs measurement first.**

**Risk.** Medium. `seek_sentinel` is public-trait-driven (`SeekSentinel`); changing the seek
mechanism must preserve "find sentinel anywhere in a noisy buffer" semantics (the bench prepends
junk + zeros, per the repo's sentinel-example convention). Do not break that.

---

## Recommended ordering (quick wins first)

1. **Step 1 (winnow `be_*` → `from_be_bytes`)** — EASY, body-only, biggest expected flat win, helps
   every row, zero regression risk to nested/native. **Do this first and re-bench before anything
   else** — it may close most of the flat gap on its own.
2. **Step 3 (`#[inline]` on `decode_value`/`finalize`)** — trivial, ship alongside step 1 (won't
   move this bench, helps downstream).
3. **Step 2b (infallible `finalize` for all-default structs)** — low risk, partial Tax-B win.
4. Re-bench. If flat still trails prost: **Step 2a (in-place direct `decode_value`)** — the real
   Tax-B fix, but it touches derive output; gate behind the existing test suite and verify
   nested/native do not regress.
5. **Step 4 (bounds-check elision)** — only after 1–2, only if a micro-bench shows the double check;
   small and fiddly.
6. **Step 6 (framed seek)** — only for framed rows, only after measuring whether `Finder`
   setup/deref is the culprit.
7. **Step 5 (`Vec` checkpoint)** — explicitly **skip**; high risk to the nested win, ~zero reward.

## How close to prost can we realistically get

- **flat (72 → target ≤61):** Step 1 alone likely reaches ~62–66ns (winnow shift-loop is the
  dominant tax). Step 2a should close the remainder and may dip below prost, since KLV's 1-byte
  keys/lengths are cheaper to parse than prost's varints once the int-decode tax is gone. Reaching
  **parity-to-slightly-better than prost (61) and beating manual (85)** is realistic.
- **nested (189, already beats prost 234):** Step 1 makes the leaf reads cheaper → expect ~170–180ns,
  approaching `manual` (158). **No regression risk** from steps 1/3. Keep step 5 hands-off.
- **native (281, prost 248, manual 200):** Step 1 helps the int-backed fields; the residual gap is
  the native-type **validation** (`char::try_from`, `NonZeroU32::new`, date/time range checks in
  `src/traits/native.rs`) which is inherent to decoding into real native types and is *not* a
  defect — prost decodes into a generated struct of raw scalars and pays the conversion elsewhere
  (and carries WKT sub-messages, see `KLV_VS_PROTOBUF.md`). Realistic target ~250–260ns (≈ prost);
  beating manual (200) is unlikely without giving up the native-type validation, which is the
  feature, not a bug.

**Bottom line:** Step 1 is the high-value, low-risk move and should be measured in isolation first.
Steps 2a/2b are the structural follow-up to match prost's in-place decode. Everything else is
secondary and gated on measurement. Do not touch the `Vec` repeated path or the streaming
`resume_partial` contract — those are where tinyklv already wins.
