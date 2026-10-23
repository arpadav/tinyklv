# First packet

`#[derive(Klv)]` reads a struct from KLV bytes and writes it back. It needs:

| on | attribute | sets |
|---|---|---|
| the struct | `key(..)`, `len(..)` | how every key and length is read and written |
| each field | `key` | the field's key byte |
| each field | `dec`, `enc` | how the field's value is read and written |

## Example

Every later page builds off this struct and shows only its change.

```rust
{{#include ../../../../examples/book/basics/book_basics_first_packet.rs:sensor}}
```

> [!NOTE]
> The `*` before a field's encoder is a sigil: it hands the field to the encoder by value. See [Sigils](../2-codecs/03-sigils.md).
>
> The key and length encoders take no sigil, since they already receive their value. A sigil there is a compile error.

## Byte representation

```klv
k 01 l 01 v 2A      // seq  = 42
k 02 l 02 v 09 2E   // temp = 2350
```

| | `seq` | `temp` |
|---|---|---|
| key | <span class="klv">k 01</span> | <span class="klv">k 02</span> |
| length | <span class="klv">l 01</span> | <span class="klv">l 02</span> |
| value | <span class="klv">v 2A</span> | <span class="klv">v 09 2E</span> |
| read as | `u8`: 42 | `be_u16` (big-endian): 2350 |

## Run this example

```sh
cargo run --example book_basics_first_packet
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** set temp to 2351, then run it again.

<details>
<summary>Answer</summary>

It prints the new bytes, then its first `assert_eq!` fails: the expected bytes are still 2350's.

```text
Sensor { seq: 42, temp: 2351 } -> [01, 01, 2A, 02, 02, 09, 2F]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/basics/book_basics_first_packet.rs:all}}
```

</details>

**Next:** [Out-of-order keys](./02-keys-not-positions.md)
