# 09 - BER-Keyed Containers

Fixed-width keys cap your namespace at 256 or 65,536 identifiers. When
you need a larger, extensible tag space - or you are implementing a
protocol that already uses BER-OID encoding - swap the key codec for
`tinyklv::dec::ber::ber_oid` / `tinyklv::enc::ber::ber_oid`.

## What it teaches

- Multi-byte BER-OID keys (continuation-bit encoding)
- Keys that need more than one byte in the wire but compare as `u64` in
  Rust
- Why BER-OID is compact for small values and efficient for large ones

## The code

```rust,no_run
{{#include ../../../examples/09_ber_keyed.rs}}
```

## Run it

```sh
cargo run --example 09_ber_keyed
```

## Key takeaway

BER-OID is just another codec. Dropping it into `key(dec = ..., enc =
...)` changes the wire format without touching any field types on the
Rust side. Encodable tag values up to `u64::MAX`.

Next: [10 - Defaults and Init](./10-defaults-and-init.md).
