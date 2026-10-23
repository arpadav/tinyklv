# Break on a field value

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [6.2 Break function](./02-decide-per-key.md)

## Default: every value is kept

The decoder keeps every value a field's decoder returns. Page 6.2's function sees only the key, before the value is read.

## A value that means a fault

Say the sensor writes temp 0xFFFF when it fails. `be_u16` reads it as 65535: 655.35 °C. A field's `break_on` names a function that checks the value once it is read:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        break_on = check_temp,
     )]
     /// Temperature, in 0.01 °C
     temp: u16,
 }
+
+/// Rejects the sensor's fault value
+fn check_temp(temp: &u16) -> BreakType {
+    if *temp == 0xFFFF {
+        BreakType::Abort("temp: the sensor faulted")
+    } else {
+        BreakType::Proceed
+    }
+}
```

| `BreakType` | the decoder then |
|---|---|
| `Proceed` | keeps the value |
| `Skip` | drops the value, as if its item were missing |
| `Done` | keeps the value, and stops |
| `Abort` | stops, and returns an error carrying its message |

```klv
k 01 l 01 v 2A      // 1. seq = 42
k 02 l 02 v FF FF   // 2. be_u16 reads 65535; check_temp returns Abort
                    // 3. decode_value returns the error
```

Only the decoder calls `check_temp`: the encoder writes 0xFFFF as it is.

> [!NOTE]
> **A field with a `break_on` is written last.** The encoder writes it after every other field, since a decoder that stops at it never reads what follows. Here temp is already last, so its bytes stay where page 1.1 has them. With a `break_on` on seq instead, the encoder writes temp's item first, then seq's.

## Run this example

```sh
cargo run --example book_control_field_break_on
```

```text
Sensor { seq: 42, temp: 65535 } -> [01, 01, 2A, 02, 02, FF, FF]
invalid `Sensor::temp` value (its `break_on` returned `Abort`)
invalid temp: the sensor faulted
```

**Try:** in `check_temp`, make the fault value return `BreakType::Skip`.

<details>
<summary>Answer</summary>

`Skip` drops temp, and a Sensor needs it, so the error changes and the last `assert!` fails:

```text
Sensor { seq: 42, temp: 65535 } -> [01, 01, 2A, 02, 02, FF, FF]
invalid `Sensor::temp` value (required: missing from the packet, or its decoder rejected it, which `strict` reports; to allow this, make the field an `Option` or give it a `default`)
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/control/book_control_field_break_on.rs}}
```

</details>

**Next:** [Draining frames](../7-streams/01-many-frames.md)
