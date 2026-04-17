# Encoder Dispatch: Why `EncodeAs` Instead of Autoref-Deref

Postmortem on an abandoned experiment, and the rationale for the
current `EncodeAs`-trait design. For current usage, see
[Encoder dispatch sigils](../reference/attributes.md#encoder-dispatch-sigils).

## Problem

`#[derive(Klv)]` codegen emits one call-site per field encoder. Some
encoders take `&T`, others want a borrowed slice form (`&str`, `&[T]`),
and primitives are cheapest when passed by value. We wanted the macro
to pick the right shape without clones or heap allocation, and without
making the user write a different attribute per field type.

## Abandoned approach: dtolnay-style autoref specialization

The autoref-deref trick works when the thing being specialized is a
**method** on a wrapper type - method resolution prefers the inherent
impl over a `Deref`-reached one. We attempted to lift this to `Fn`
traits: two wrappers, one implementing a trait for `Fn(&T) -> O`, the
other (via `Deref`) for `Fn(T) -> O`.

It does not work. `Fn` traits are **invariant** over their argument
types. Method resolution does not re-examine trait impls after an
initial candidate is selected, and trait-based dispatch has no
equivalent of method-resolution priority. The autoref-deref pattern is
specific to inherent methods on concrete types - not generalizable to
trait selection on closure-like values.

## Second iteration: three lexical sigils with `Clone::clone`

An earlier revision of the proc macro accepted three sigils:

| Sigil | Emitted |
|-------|---------|
| none | `func(&self.field)` |
| `&` | `func(Clone::clone(&self.field))` |
| `*` | `func(&*self.field)` |

This got the call shapes right but:

- `&` cloned every `Copy` primitive (`Clone::clone(&u32)` is waste the
  optimizer *usually* removes, but the IR churn is real and the intent
  reads backwards).
- `&` allocated for `String` (`Clone::clone(&String)` → new heap
  buffer) when the encoder only needed `&str`.
- `*` meaning "`&*self.field` at the call site" was counterintuitive -
  users read `*func` as dereferencing the encoder, not as
  pre-dereferencing the value.
- Three shapes for what is, at the value level, one decision
  (how to hand the owned field to the encoder).

## Current approach: `&` sigil routed through `EncodeAs`

Rust specialization is still unstable, but we do not need it - a
dedicated trait with a GAT for the borrowed form resolves the dispatch
cleanly:

```rust
pub trait EncodeAs {
    type Borrowed<'a> where Self: 'a;
    fn encode_as(&self) -> Self::Borrowed<'_>;
}
```

The crate ships impls for the shapes where no-sigil `&T` is wrong:

| `T` | `Borrowed<'_>` | `encode_as(&self)` |
|-----|----------------|--------------------|
| primitives (`u8..u128`, `i…`, `usize`, `f32/f64`, `bool`, `char`) | `Self` | `*self` (by value, no clone) |
| `String` | `&str` | `self.as_str()` |
| `Vec<T>` | `&[T]` | `self.as_slice()` |
| `Cow<'_, str>` / `Cow<'_, [T]>` | `&str` / `&[T]` | `self` |
| `Box<T>`, `Rc<T>`, `Arc<T>` (`T: ?Sized`) | `&T` | `&**self` |

Codegen collapses to two templates:

| Sigil | Emitted (required) | Emitted (`Option<T>`) |
|-------|--------------------|------------------------|
| none | `func(&self.field)` | `func(__val)` |
| `&` | `func(EncodeAs::encode_as(&self.field))` | `func(EncodeAs::encode_as(__val))` |

No `Clone::clone` anywhere in the generated encode path. Primitives
compile down to a by-value pass; `String`/`Vec<T>`/`Cow`/`Box`/`Rc`/`Arc`
compile down to the zero-cost borrow. Custom types are opt-in - a
`Copy` enum gets it with a one-liner returning `*self`; otherwise the
user stays on the no-sigil form and deref coercion handles
`&String → &str`, `&Vec<u8> → &[u8]` at the call site as before.

See `src/traits/coerce.rs` for the trait and its impls,
`impl/src/ast/types.rs` (`XcoderSigil`, `SiguledXcoder`) for the
two-variant sigil enum, and `impl/src/expand/encode_impl.rs` for the
per-sigil `quote_spanned!` blocks.

## Why not a blanket `impl<T> EncodeAs for T`?

A blanket `fn encode_as(&self) -> &Self` would conflict with the
specialized impls (`String → &str`, `Vec<T> → &[T]`, etc.) - Rust has
no specialization to break the tie. Making users opt-in for custom
types is a small papercut compared to the alternative (forcing every
field through the slow path, or requiring negative impls). The blanket
is deliberately omitted; `EncodeAs` is only consulted when the user
writes `&` in the attribute.
