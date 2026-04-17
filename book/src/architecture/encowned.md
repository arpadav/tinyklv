# EncOwned: Autoref-Deref Specialization Attempt

Personal notes on the attempt to automatically dispatch between `fn(&T) -> Vec<u8>`
and `fn(T) -> Vec<u8>` encoder signatures using dtolnay's autoref-deref trick.

**Outcome:** Abandoned. Stable Rust cannot do this. Replaced with `enc_owned!` macro.

---

## Goal

The `#[derive(Klv)]` proc macro generates encoder calls as `encoder(&self.field)`.
This works for `fn(&T) -> Vec<u8>` but fails for `fn(T) -> Vec<u8>`.

Wanted: proc macro emits one call site that resolves to either signature automatically.

## Approach: dtolnay Autoref-Deref Specialization

Based on [dtolnay's autoref specialization pattern](https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md).

Two wrapper structs - one with an inherent method (higher priority in method resolution),
one reached via `Deref` (lower priority fallback):

```rust
pub mod __enc {
    use std::ops::Deref;

    /// Primary wrapper - inherent method matches `Fn(&T) -> O`
    pub struct EncCall<F>(pub F);

    /// Fallback wrapper - reached via Deref, matches `Fn(T) -> O`
    pub struct EncCallOwned<F>(pub F);

    // Deref from EncCall → EncCallOwned so method resolution
    // tries EncCall::call first, then falls back to EncCallOwned::call
    impl<F> Deref for EncCall<F> {
        type Target = EncCallOwned<F>;
        fn deref(&self) -> &Self::Target {
            // Same repr - single-field newtype
            unsafe { &*(self as *const EncCall<F> as *const EncCallOwned<F>) }
        }
    }

    // Priority 1: encoder takes &T
    impl<F, T, O> EncCall<F>
    where
        F: Fn(&T) -> O,
    {
        pub fn call(&self, v: &T) -> O {
            (self.0)(v)
        }
    }

    // Priority 2 (via Deref): encoder takes T by value
    impl<F, T, O> EncCallOwned<F>
    where
        F: Fn(T) -> O,
        T: Clone,
    {
        pub fn call(&self, v: &T) -> O {
            (self.0)(v.clone())
        }
    }
}
```

Proc macro would generate:

```rust
// Instead of: #value_encoder(&self.#name)
// Emit:
::tinyklv::__enc::EncCall(#value_encoder).call(&self.#name)
```

Method resolution should try `EncCall::call` (inherent, `Fn(&T)`) first.
If encoder is `fn(T)`, that fails, and Deref fallback resolves to `EncCallOwned::call`.

## Why It Failed

### Problem 1: Fn trait invariance breaks &String → &str coercion

```rust
fn from_string_utf8(input: &str) -> Vec<u8> { input.as_bytes().to_vec() }
```

The proc macro wraps a `String` field, so the call site is:

```rust
EncCall(from_string_utf8).call(&self.label)  // label: String
```

This requires `from_string_utf8: Fn(&String) -> Vec<u8>`, but `from_string_utf8`
is `fn(&str) -> Vec<u8>`. At a direct call site, Rust auto-derefs `&String → &str`.
But inside a `Fn(&T)` trait bound, `T` is inferred as `String`, and `Fn(&String)`
does NOT match `fn(&str)` - **Fn traits are invariant in their input types**.

#### Attempted fix: Borrow constraint

```rust
impl<F, T, B, O> EncCall<F>
where
    F: Fn(&B) -> O,
    T: std::borrow::Borrow<B>,
    B: ?Sized,
{
    pub fn call(&self, v: &T) -> O {
        (self.0)(v.borrow())
    }
}
```

This fixes `String`/`str` coercion. But introduces Problem 2.

### Problem 2: Method resolution does NOT fall through

The autoref-deref trick works when the **receiver type** (`&self` vs `&&self`)
disambiguates. Here, both wrappers take `&self` - the dispatch relies on
**argument type inference** of the `F` bound.

Rust's method resolution:

1. Find candidates by receiver type (inherent methods first, then Deref chain)
2. `EncCall::call` is inherent → it's the first candidate
3. Rust **commits** to this candidate
4. Then attempts type inference on `F: Fn(&B) -> O` with `T: Borrow<B>`
5. If inference fails → **compile error**, not fallback to `EncCallOwned::call`

The trick requires the compiler to see that `EncCall::call` is not applicable
and *then* try the Deref target. But it commits before checking bounds fully.

This is a fundamental limitation: the autoref-deref trick dispatches on
**receiver type** (number of `&` wrappers), not on **generic bound satisfaction**.

### Problem 3: Closure vs fn-pointer ambiguity

Even if fallthrough worked, closures and function pointers have different `Fn` impls.
A bare `fn(T) -> O` is both `Fn(T) -> O` and `Fn(&T) -> O` (for some inference paths),
creating ambiguous resolution.

## Solution: `enc_owned!` macro

Explicit opt-in. No magic. User wraps owned-taking encoders:

```rust
#[macro_export]
macro_rules! enc_owned {
    ($encoder:path) => {
        |v| $encoder(::core::clone::Clone::clone(v))
    };
}
```

Usage:

```rust
fn encode_priority_owned(v: Priority) -> Vec<u8> { /* ... */ }

#[derive(Klv)]
#[klv(/* ... */)]
struct Packet {
    #[klv(key = 0x01, dec = decode_priority, enc = tinyklv::enc_owned!(encode_priority_owned))]
    priority: Priority,
}
```

The macro expands to a closure `|v: &Priority| encode_priority_owned(v.clone())`,
which satisfies the proc macro's `encoder(&self.field)` call pattern.

Requires `T: Clone`. Proc macro already supports `XcoderLike::Macro` variant
in the attribute parser, so `enc = tinyklv::enc_owned!(...)` works out of the box.

## Takeaway

Stable Rust cannot dispatch on function argument types at a single call site.
The autoref-deref trick only works for **receiver-based** dispatch (inherent vs Deref).
Generic bound satisfaction happens *after* candidate selection, not during.

For dual-signature support, the options are:
1. **`enc_owned!` macro** - explicit, zero-cost, works today ✓
2. **Specialization** - unstable, `min_specialization` doesn't cover this
3. **Proc macro detection** - would need to resolve types at macro expansion time (impossible without compiler integration)
