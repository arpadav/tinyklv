# Plan: per-type `const SIZE` on `EncodeValue` (encode fast-path "for free")

## Context

`size(exact = N)` is the encode win: it selects the `FixedWidth` strategy
(`impl/src/expand/encode_impl/strategy.rs:75`) - write key, a **constant** length,
the value, no length back-patch - ~4.5–5.4× over hand-written encode on the
`rich`/`compound`/`simple` benches (`benches/bench.csv`).

Two questions drove this plan:

1. **Can the hint help the DECODE path?** Investigated: **no.** The wire always
   carries the `len` prefix; the decoder must parse it, bounds-check
   (`input.eof_offset() < len`), and `take(len)` regardless
   (`impl/src/expand/decode_impl/decode_partial_gen.rs:413,572`). A compile-time
   width cannot skip the runtime `len` read. `exact_width()` is referenced only
   under `encode_impl/`; the `Vec<T>` decode prealloc uses `eof_offset().min(NUM_ELEM)`,
   not `size`. **No decode work to do.**

2. **Push the width into the type system via the trait.** Today the macro reaches
   `FixedWidth` only at *expansion time*: an explicit `size(exact = N)` literal, or
   the built-in primitive registry (`width.rs`). Types that impl `EncodeValue` but
   the macro can't measure - `Ipv4Addr`, `DateTime<Utc>`, `Duration`, `Ipv6Addr`, …
   in `src/traits/native.rs` - force a hand-written `size(exact = 4)` on **every**
   field. This plan lets each such type *declare its own* width once, so the fast
   path is picked up with zero annotation.

   **Primitives (`u8`…`f64`) are unaffected and need nothing**: they don't impl
   `EncodeValue` (they encode via `codecs::` free fns) and already get `FixedWidth`
   from the macro registry. The gap is *only* the `EncodeValue`-typed std/chrono
   types - exactly what the const reaches.

**Precedent (the strongest justification):** `DecodeValue` already carries a
per-type, defaulted, optimization-only associated const -
`NUM_ELEM = BUDGET / size_of::<Self>()` (`src/traits/dec/mod.rs:65`) - that feeds
`Vec::with_capacity` and never changes decoded bytes. `EncodeValue::SIZE` is its
encode twin: a width const that drives the fast path / `Vec::reserve` and never
changes the wire when `Unknown`. Adding it makes the two value traits *more*
symmetric, not less.

User decisions (locked):
- Carrier: **`EncodeValue`** (emitted code calls `Self::SIZE` + `const fn` methods,
  compiler-folded). Not a companion trait.
- Ambition: **full const-guarded fast-path** (skip back-patch), not reserve-only.
- Value type: **flat `tinyklv::Size { Unknown, Exact(usize), Hint(usize) }`**,
  `const SIZE: Size = Size::Unknown` (mirrors the macro-side `SizeBytes`; helpers
  are plain `const` methods on `self`, no `Option` glue).
- `native.rs`: **stays `#[cfg(feature = "bench")]`** - ship the *mechanism*;
  benches prove it. Un-gating is a separate later decision.
- Derived `#[derive(Klv)]` structs: **default `SIZE = Size::Unknown`**. Auto
  computing a struct's width is a noted **future** extension.

## Precedence (the field annotation gates whether `SIZE` is read)

The field's *bytes axis* (`exact`/`hint`) wins over the type's `SIZE`; the type is
consulted only when the field declares **no** byte count. `SizeSpec::capacity_bytes()`
is already `Some` for both `exact` and `hint`, `None` for unset - so the gate is exact:

| field annotation                | byte width used        | `SIZE` read? |
|---------------------------------|------------------------|--------------|
| `size(exact = N)`               | `N` (FixedWidth lit)   | no           |
| `size(hint = N)`                | back-patch, reserve `N`| no           |
| `size(var)` only                | from `SIZE`            | **yes**      |
| *(no `size`)*                   | from `SIZE`            | **yes**      |

So `size(var)` + `SIZE = Exact(20)` ⇒ uses 20. This also makes the reserve
contributions mutually exclusive (a probed field has no annotation bytes), so no
double-count.

Full strategy precedence: `size(exact=N)` lit -> registry primitive ->
**`Probed` (new, reads `SIZE`)** -> back-patch.

## The crux (why this isn't just "add a const")

A proc-macro runs **before** types resolve - it **cannot read**
`<Ipv4Addr as EncodeValue>::SIZE` at expansion time. So the strategy choice for a
probed field can't be made structurally in the macro; instead the macro emits a
branch guarded by an **inline `const { }`** block (stable 1.79; crate MSRV 1.95),
and the compiler const-folds + DCEs the dead arm:

```rust
// emitted for a probed field (fallback encoder, no annotation bytes, Fixed len).
// `#t` = the field type, looking through Option<T> for optional fields.
match const { <#t as ::tinyklv::traits::EncodeValue>::SIZE.exact_width() } {
    // Exact(w) -> fast path, identical to today's FixedWidth arm
    ::core::option::Option::Some(__w) => {
        #key_encoder(#key, out);
        #len_encoder(__w, out);
        let __vstart = out.len();
        #enc_tokens(#arg, out);
        debug_assert_eq!(out.len() - __vstart, __w,
            "tinyklv: EncodeValue::SIZE disagrees with bytes written");
    }
    // Unknown / Hint -> exactly today's back-patch path, no regression
    ::core::option::Option::None => { #backpatch }
}
```

**Inline `const { }` - not a named `const __W = …` item** - is required: a named
inner const referencing the outer generic `#t` is **E0401** ("can't use generic
parameters from outer item"), which fires on any generic derived struct
(`tests/derive/advanced_generics.rs`). An inline const block inherits the
surrounding generic context and is legal. The scrutinee is const-evaluated, so
`__w` monomorphizes to a constant and the dead arm is DCE'd under `-O`; in debug
it degrades to one predictable branch (state this - don't bench debug). When
`SIZE.exact_width()` is `None` the field behaves **byte-for-byte as today**, so
this is a strict superset.

**Scope guard:** `Probed` fires **only for `fallback_enc` fields** (encoder ==
`<T>::encode_value`, so `SIZE` authoritatively describes the bytes) under a
**`LenPrefix::Fixed`** container, **only when the field declares no annotation
bytes**. For `Option<T>` fields, the inner type `T` is probed (via
`helpers::unwrap_option_type`) **inside** the existing `if let Some(ref __val)`
guard - when present, the value is exactly `w` bytes, so the fast path is correct.
A field with an explicit `#[klv(enc = custom_fn)]` is never probed (its encoder may
write a different width than the type's canonical `SIZE`).

## Changes

### 1. New value type + const methods - `src/traits/size.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// deliberately NOT #[non_exhaustive] and NOT Ord: pre-release (breaking changes
// are fine), and same-crate const fns need exhaustive matches for const-eval; an
// Exact-vs-Hint ordering is meaningless. Revisit at stabilization.
pub enum Size { Unknown, Exact(usize), Hint(usize) }

impl Size {
    #[must_use] pub const fn is_exact(self) -> bool { matches!(self, Size::Exact(_)) }
    #[must_use] pub const fn is_hint(self) -> bool { matches!(self, Size::Hint(_)) }
    /// exact wire width (drives the fast path); `Hint`/`Unknown` -> `None`
    #[must_use] pub const fn exact_width(self) -> Option<usize> {
        match self { Size::Exact(n) => Some(n), _ => None }
    }
    /// capacity contribution (drives `Vec::reserve`); `Exact`/`Hint` both count
    #[must_use] pub const fn capacity_bytes(self) -> Option<usize> {
        match self { Size::Exact(n) | Size::Hint(n) => Some(n), Size::Unknown => None }
    }
    /// `Vec::reserve` term: `capacity_bytes` with the `Unknown -> 0` default baked in
    /// (`Option::unwrap_or` is not `const`)
    #[must_use] pub const fn reserve_bytes(self) -> usize {
        match self { Size::Exact(n) | Size::Hint(n) => n, Size::Unknown => 0 }
    }
}
```

Method names **mirror the macro-side `SizeSpec`** (`exact_width`, `capacity_bytes`)
so the parse layer and the runtime layer read in lockstep. No `#[inline]` (const
fns; const-evaluated at every call site). File gets the repo module header
(`//! …` ending `//! Author: aav`), `// ---` separators, one-line `///` per variant,
and an invariant note: *a type whose `SIZE = Exact(n)` MUST write exactly `n` bytes;
a lying impl corrupts the wire (debug-asserted only).* Cross-reference
`SizeSpec`/`SizeBytes` as the twin *parse-layer* type - distinct concern, not DRY.

`src/traits/mod.rs`: `mod size; pub use size::*;` (root resolves `tinyklv::Size`
through the existing `pub use traits::*` in `src/lib.rs:16`). Also add `Size` to the
inline prelude (`src/lib.rs:68-82`, the local-import group) so hand-writers of
`impl EncodeValue` get it in scope. **There is no `src/prelude.rs`** - the prelude
is that inline module.

### 2. Trait const - `src/traits/enc.rs`

```rust
pub trait EncodeValue {
    /// Declared byte size of this type's `encode_value` output.
    /// `Unknown` (default) -> variable/unknown -> back-patch path.
    const SIZE: crate::traits::Size = crate::traits::Size::Unknown;
    fn encode_value(&self, out: &mut Vec<u8>);
}
```
Default keeps every existing impl (incl. the `Vec<T>` blanket) non-breaking. Add a
doc example showing an override (`impl EncodeValue for Rgb { const SIZE = Size::Exact(3); … }`)
and a line: *leave `SIZE` at the default `Unknown` for variable/unknown widths -
back-patch is always correct.* **Breaking:** an associated const makes
`dyn EncodeValue` illegal (E0038); no current `dyn` use exists - note it in
`CHANGELOG.md` as a breaking change.

### 3. Declare sizes on native types - `src/traits/native.rs`

Add `const SIZE` to each `EncodeValue` impl (still `bench`-gated):
`DateTime<Utc>`/`Duration` -> `Exact(8)`, `Ipv4Addr`/`NaiveDate`/`NaiveTime`/`char`/
`NonZeroU32` -> `Exact(4)`, `Ipv6Addr` -> `Exact(16)`, `bool` -> `Exact(1)`. Each equals
the bytes its `encode_value` already writes (one BE primitive). The `Vec<T>` blanket
(`enc.rs:160`) stays the default `Unknown` (variable).

### 4. Codegen - `impl/src/expand/encode_impl/`

- **`strategy.rs`**: add `FieldEncodeStrategy::Probed { len_width }`. Update the enum
  doc - it is the one variant whose write shape is *conditional* (fixed when the type
  declares an exact `SIZE`, else back-patch), so loosen the "all three produce
  byte-identical output" invariant statement. In `field_encode_strategy`, after the
  explicit-`exact` check and before the registry check: when
  `fallback_enc && matches!(len_prefix, Fixed)` and the field declares no annotation
  bytes (`size.and_then(SizeSpec::capacity_bytes).is_none()`), return `Probed`; a
  `fallback_enc` field that *did* declare bytes (i.e. `hint`) returns `Backpatch`
  (honor the hint, don't probe).
- **`field_gen.rs`**: emit the inline-`const`-guarded `match` above for `Probed`,
  reusing the existing `FixedWidth` arm body (with the `SIZE`-specific assert
  message) and `backpatch::emit_len_backpatch` for the two arms - no new write
  primitive. Probe `helpers::unwrap_option_type(ty).unwrap_or(ty)` so `Option<T>`
  fields probe `T` inside their `if let Some` guard.
- **`reserve.rs`**: change `gen_reserve_hint` to return `proc_macro2::TokenStream`
  (keep `pub(super)`; update its doc - it now emits a const-expression, not a
  literal). Per-field term priority: `fixed_value_width(ty)` lit ->
  `attrs.size.capacity_bytes()` lit -> (probed fields only)
  `<#t as ::tinyklv::traits::EncodeValue>::SIZE.reserve_bytes()` -> `0`. Build the sum
  with `quote!`, **each term parenthesized** (`quote! { 0usize #( + (#terms) )* }`),
  not `Iterator::sum`. `Hint` finally earns its keep here.
- **`mod.rs`** / **`frame_gen.rs`**: `out.reserve(#reserve)` interpolates the token
  expr fine; **`gen_encode_frame`'s `reserve: usize` param becomes a `TokenStream`**
  and its `frame_gen.rs:59` arithmetic site must emit `#reserve` **parenthesized**
  (`(#reserve) + 2 * #width + #sentinel.len()`).

### 5. Symmetry doc - `src/traits/mod.rs`

After the trait-architecture table, one line: *both value traits carry a defaulted,
optimization-only const - `DecodeValue::NUM_ELEM` (prealloc element budget) and
`EncodeValue::SIZE` (declared wire width); neither affects the bytes exchanged.*

## Soundness

- `Unknown`/`Hint` (`exact_width() == None`) path is byte-identical to today's
  back-patch -> no behavioural change for any existing type.
- A type whose `SIZE` *lies* (`Exact(n)`, writes ≠ `n`) corrupts the wire - caught by
  the fast-arm `debug_assert_eq!` in debug/tests, **silent in release**. New vs
  today: the responsibility moves from the KLV-struct author (who wrote the
  attribute) to the *field-type* author (who declared `SIZE`, possibly a different
  crate). Native impls are trivially correct. Note this ownership shift.
- Const-fold/DCE is an optimization, not correctness: even un-optimized, `match` on
  an inline-`const` scrutinee is one predictable branch.

## Tests (extensive - folders, custom types)

- `size.rs` unit tests: `is_exact`/`is_hint`/`exact_width`/`capacity_bytes`/
  `reserve_bytes` over `Unknown`/`Exact`/`Hint`.
- new `tests/derive/<const_size>.rs` with real custom structs/enums (per repo
  convention, not bare primitives):
  - custom type `SIZE = Exact(N)` as a field with **no** `size` annotation ->
    byte-identical to the same struct *with* `size(exact = N)`, plus round-trip.
  - `Hint(N)` custom type -> takes back-patch **and** widens the reserve.
  - `Unknown` custom type -> back-patch round-trip.
  - mixed struct (sized + unsized fields).
  - `Option<NativeType>` field -> probed fast path, byte-identical to a sized variant.
  - precedence: `size(exact = M)` overrides `SIZE = Exact(N)` (M≠N ⇒ M wins);
    `size(var)` + `SIZE = Exact(20)` ⇒ 20; `size(hint = K)` + `SIZE = Exact(N)` ⇒
    back-patch + reserve K (no probe).
  - lying `SIZE` -> `#[should_panic]` under `debug_assert`.
  - generic `struct Foo<T: EncodeValue>` with a probed field **compiles** (E0401
    regression guard).
  - `Vec<T>` blanket: pin `<Vec<NativeType>>` field still back-patches / round-trips.

## Benches (prove "for free")

In `benches/suite/records/{rich,compound}.rs`, drop the now-redundant
`size(exact = N)` on native-type fields and rely on `SIZE`; numbers should **match**
the annotated fast-path (same emission, now automatic). High `sample_size` per repo
convention; record before/after in `FINDINGS.md`.

Optional, measured: extend the `Vec<T>` blanket `encode_value` to
`out.reserve(self.len() * n)` when `T::SIZE` is `Exact(n)` - a free reserve win on
the `sensors: Vec<_>` compound-encode hot path (mirrors the decode-prealloc win).
Keep only if it benches positively.

## Docs

- `encoding-model.md` + book "encoding model": add the `EncodeValue::SIZE` row to
  the attribute/path table and the precedence note above; net-neutral word budget on
  touched sections.
- `src/traits/enc.rs` `SIZE` doc + override example; `src/traits/size.rs` item docs
  (one-line per item); `CHANGELOG.md` unreleased (incl. the `dyn` breaking note).

## Future (noted, spec'd, not built)

Derived `#[derive(Klv)]` structs could emit a computed `const SIZE` - this is the
lever on the *actual* remaining encode bottleneck (the nested-record `Backpatch`
`copy_within`/`truncate` memmove), and the native-type mechanism here is its
prerequisite (a struct is exact iff all its *framed* fields are exact, which now
includes native types). Spec for that step: a `const fn Size::combine(a, b)` folding
per-field **framed** widths (key + len + value) - `Exact+Exact=Exact(sum)`, any
`Hint`->`Hint`, any `Unknown`->`Unknown`; the struct's `SIZE` = combine over fields.
Deferred per user (structs default `Unknown`).

## Verification

1. `cargo build`, `cargo build --features bench`, **`cargo build --no-default-features`**
   (prove the enum/const aren't accidentally feature-gated).
2. `cargo test --features bench` - round-trip, byte-identity, precedence, lying-SIZE,
   generic-compile, `Vec` blanket tests green.
3. `cargo expand` on a probed struct - confirm the inline `const { … }` `match`; on
   `--release` `--emit=llvm-ir`, grep that **no `tinyklv::…::Size` symbol survives**
   (fully const-folded).
4. `cargo bench` `rich`/`compound` with annotations removed - parity with recorded
   fast-path numbers.
5. `cargo clippy --all-targets --features bench` clean.

## Critical files

- new `src/traits/size.rs`; `src/traits/mod.rs` (`mod`+`pub use`+symmetry doc);
  `src/lib.rs` (prelude entry - inline module, **not** `src/prelude.rs`)
- `src/traits/enc.rs` (trait const + doc), `src/traits/native.rs` (declare sizes)
- `impl/src/expand/encode_impl/{strategy.rs,field_gen.rs,reserve.rs,mod.rs,frame_gen.rs}`
- `benches/suite/records/{rich,compound}.rs`, `FINDINGS.md`, `encoding-model.md`,
  `CHANGELOG.md`, new `tests/derive/<const_size>.rs`
