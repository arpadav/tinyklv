# Tutorial 19 - Unknown items

An item whose key no field declares is skipped by its length. The container
`unknown(dec, enc)` codec reads it instead, and a `#[klv(unknown)]` field keeps
it, so encode writes it back:

- `dec` takes the key first, as a `lengthed` decoder takes the length:
  `fn(K) -> impl Fn(&mut S) -> tinyklv::Result<U>`.
- `enc` writes a kept value, `fn(&U, &mut Vec<u8>)`, with the field sigils.
- The field is a `Vec<(K, U)>`: one per struct, with no other setting.

## The decoder contract

The decoder gets the item as the header left it:

- in key + len, the main stream after the key: it reads the
  length itself, and `enc` writes it back;
- beside `keylen(..)`, a `Sliced(n)` item's `n` bytes, or the main stream for
  an `Inline` one: `enc` writes the value alone.

On the main stream, an item cut short must consume the input to its end
(`input.finish()`): the loop reads that as truncation, and streaming waits for
the rest. Failing anywhere else, or on a kept `Sliced` item, is an error
naming the `#[klv(unknown)]` field, or the struct's item without one. The
decoder advances only within the slice it is given, as `input.finish()` or
`*input = &input[input.len()..]` do: in a key + len struct, one that sets
`*input = &[]` and returns `Ok` panics with debug assertions or overflow checks
on. An item that consumes no byte, key included, is an error.

## Drop or keep

Without the field, each decoded value is dropped, `enc` may be left out, and a
`keylen` struct skips a `Sliced` unknown item undecoded. With it, encode writes
the kept items in decode order after the fields, before any `break_on` one.

Run this example: `cargo run --example book_19_unknown`

```rust
{{#include ../../../../examples/book/advanced/book_19_unknown.rs}}
```

## Speed

Opting in costs speed even when no unknown item arrives. The loop's `_` arm
passes the kept field (or, when the unknown decoder is not inlined, the cursor)
to a call that is not inlined, so the fields decoded so far live in stack
memory for the whole loop: each known arm loads and stores its field, and the
`u16` arm below takes 12 instructions instead of 6. A struct without
`unknown(..)` is unaffected. Each listing is that arm decoding `02 02 22 22`
(key, length, a big-endian `u16`) in an eight-field struct, not the benched one
below, built by rustc 1.95.0 (LLVM 22.1.2) for `x86_64-unknown-linux-gnu` with
`-C opt-level=3 -C lto=fat -C codegen-units=1` and no `-C target-cpu`.

```x86asm
; no unknown(..): the field stays in registers, 6 instructions
cmp    r8b,0x2
jb     14720
movzx  r10d,WORD PTR [rsi]
rol    r10w,0x8
mov    r14b,0x1
jmp    14720
```

```x86asm
; unknown(..) and a kept field: 12 instructions
cmp    cl,0x2
jb     14d05
movzx  edx,WORD PTR [r12]
rol    dx,0x8
cmp    cl,0x2
cmovb  dx,WORD PTR [rsp+0x52]
movzx  ecx,WORD PTR [rsp+0x50]
mov    si,0x1
cmovae cx,si
mov    WORD PTR [rsp+0x50],cx
mov    WORD PTR [rsp+0x52],dx
jmp    14b72
```

Decode time of eight fixed-width fields, measured with `criterion` (`benches/framing_modes.rs`,
the same flags; the two unknown items carry 3 and 2 bytes):

| struct | no unknown items | two unknown items |
|---|---|---|
| (a) no `unknown(..)` | 10.1 ns | 11.3 ns |
| (b) `unknown(..)`, no kept field | 13.9 ns | 24.4 ns |
| (c) `unknown(..)` and a kept field | 16.3 ns | 39.1 ns |
