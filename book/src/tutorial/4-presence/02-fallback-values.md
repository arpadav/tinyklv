# Default values

**Needs:** [4.1 Optional fields](./01-optional-fields.md)

## A missing field

Page 4.1 makes a missing temp `None`, so every use of temp must handle `None`. When a missing temp has a value it should mean, give it that value instead.

## The type's default

`default` gives a missing temp its type's `Default`:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        default, // a missing temp is u16's default: 0
     )]
     /// Temperature, in 0.01 °C
     temp: u16,
```

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
                    // 2. the input ends with no key 02: temp takes its default, 0
```

temp is a plain `u16`, so the encoder always writes it, 0 included:

```klv
k 01 l 01 v 2A      // seq  = 42
k 02 l 02 v 00 00   // temp = 0
```

To leave that item out, see [Skipping defaults](./03-skipping-defaults.md).

## Another value

Give `default` an expression of the field's type, for any other value:

```rust,ignore
default = 2000, // a missing temp is 20.00 °C
```

> [!NOTE]
> **Two attributes named `default`.** On a field, `default` is the value of a missing item. On the container, `default(..)` names the codecs of every field of a type ([1.4 Default codecs](../1-basics/04-default-codecs.md)).
>
> **A temp that fails to decode.** As on [page 4.1](./01-optional-fields.md), the decoder drops an item whose value does not decode, so temp takes its default, as if key 02 were missing.

## Run this example

```sh
cargo run --example book_presence_default_value
```

```text
Sensor { seq: 42, temp: 0 } -> [01, 01, 2A, 02, 02, 00, 00]
```

**Try:** give temp a default of 2000, then run it again.

<details>
<summary>Answer</summary>

It prints the same line, then its last `assert_eq!` fails: `wire` has no key 02, so temp now decodes as 2000.

```text
assertion `left == right` failed
  left: Sensor { seq: 42, temp: 2000 }
 right: Sensor { seq: 42, temp: 0 }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/presence/book_presence_default_value.rs}}
```

</details>

**Next:** [Skipping defaults](./03-skipping-defaults.md)
