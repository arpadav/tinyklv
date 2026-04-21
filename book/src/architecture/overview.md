# Architecture Overview

`tinyklv` is two crates in a workspace:

- `tinyklv` - the runtime library: traits, built-in codecs, prelude.
- `tinyklv-impl` - the `#[derive(Klv)]` proc-macro.

## Layering

```text
          user code
              │
              ▼
   ┌──────────────────────┐
   │  #[derive(Klv)]      │    compile time (tinyklv-impl)
   │   expands to ≤ 4     │
   │   trait impls        │
   └──────────┬───────────┘
              │
              ▼
   ┌──────────────────────┐
   │  tinyklv traits      │    runtime (tinyklv)
   │  + built-in codecs   │
   └──────────┬───────────┘
              │
              ▼
   ┌──────────────────────┐
   │      winnow 1.x      │    runtime
   └──────────────────────┘
```

User code calls methods from the prelude. Those methods are provided
by trait impls that the derive macro emits. Each impl is a thin wrapper
around winnow parser combinators.

## Runtime responsibilities

- **Trait definitions** for `EncodeValue`, `EncodeFrame`, `DecodeValue`,
  `DecodeFrame`, `SeekSentinel`, `RepeatedDecode`, `BreakCondition`,
  `IntoKlv`.
- **Built-in codecs** - binary, BER, string (`codecs/binary.rs`,
  `codecs/ber/`, `codecs/string.rs`).
- **Prelude** - anonymous re-exports so method-style calls resolve.

## Proc-macro responsibilities

- **Attribute parsing** (`impl/src/ast/`).
- **Field analysis** - match each field to a key, a pair of codec paths,
  and any modifiers (`default`, `var`, `repeated`, `break`).
- **Code generation** (`impl/src/expand/`) - emit the four trait impls
  inside an anonymous `const _: () = { ... }` block to keep generated
  statics out of the user's module scope.
- **Generic preservation** - `syn::Generics::split_for_impl()` threads
  the user's type parameters and where-clauses through every generated
  impl verbatim.

## Stream abstraction

The generated code is parameterised on the container's `stream = ...`
type. Any `winnow::Stream` works; `&[u8]` is the idiomatic choice and
the default in every example. Custom stream types (a buffered reader,
a bit-stream adapter) drop in with no derive changes.
