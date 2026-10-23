# Skipping defaults

**Needs:** [4.2 Default values](./02-fallback-values.md)

## Every field is written

The encoder writes every field, even one at its default. Page 4.2's temp of 0:

```klv
k 01 l 01 v 2A      // seq  = 42
k 02 l 02 v 00 00   // temp = 0
```

## No item at the default

Protobuf, for one, writes no item for a field at its default: the decoder fills it back in. `skip_if_default` does both:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        skip_if_default, // no item when temp is 0
     )]
```

```klv
k 01 l 01 v 2A      // 1. write seq
                    // 2. temp is 0, its type's default: write no item
```

The decoder reads the missing temp as 0, as page 4.2's `default` does.

## A function as the test

`skip_if` names a function that gets the field and returns `true` to skip it:

```rust,ignore
/// Whether temp is room temperature, 20.00 °C
fn is_room(temp: &u16) -> bool {
    *temp == 2000
}
```

A missing temp still decodes to 0, so give it the skipped value as its `default`:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        skip_if = is_room, // no item at 20.00 °C
+        default = 2000,    // so a missing temp is 2000, not 0
     )]
```

```klv
k 01 l 01 v 2A      // 1. write seq
                    // 2. is_room(2000) returns true: write no item
```

A container `default(..)` ([page 1.4](../1-basics/04-default-codecs.md)) takes either one too, for every field of its type.

<details>
<summary>Full example: a function</summary>

```rust
{{#include ../../../../examples/book/presence/book_presence_skip_if_fn.rs}}
```

</details>

> [!NOTE]
> **An `Option` field.** The test takes the whole `Option`, whose default is `None`. So `None` writes no item, but `Some(0)` is written.
>
> **A float at -0.0.** -0.0 equals 0.0, so `skip_if_default` writes no item for it, and the decoder fills in 0.0: the sign is lost.

## Run this example

```sh
cargo run --example book_presence_skip_if
```

```text
Sensor { seq: 42, temp: 0 } -> [01, 01, 2A]
```

**Try:** set temp to 1, then run it again.

<details>
<summary>Answer</summary>

Only 0 is skipped. It prints temp's item again, then its first `assert_eq!` fails:

```text
Sensor { seq: 42, temp: 1 } -> [01, 01, 2A, 02, 02, 00, 01]
```

</details>

<details>
<summary>Full example: the type's default</summary>

```rust
{{#include ../../../../examples/book/presence/book_presence_skip_if.rs}}
```

</details>

**Next:** [One-of fields](./04-one-of-several-keys.md)
