# Break function

**Needs:** [6.1 Break on a key](./01-stop-on-a-key.md)

## A key stops the decoder

On page 6.1, `break_on` names a key. The decoder stops right after it, and leaves the key's length and value in the input.

## A function decides

`break_on` can name a function in place of a key. The decoder calls it after every key, with the input that follows. It may read on, then returns a `BreakType`. Here it reads FF's length and value before it stops:

```diff
 use tinyklv::prelude::*; // Klv + its traits
+use tinyklv::winnow::binary::length_take;

 #[derive(Klv, Debug, PartialEq)]
 #[klv(
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
     len(
         dec = decb::u8_as_usize,
         enc = encb::u8_from_usize,
     ),
+    break_on = check_key,
 )]
```

```diff
     /// Temperature, in 0.01 °C
     temp: u16,
 }
+
+/// Stops at key 0xFF, after its length and value
+fn check_key(key: &u8, input: &mut &[u8]) -> BreakType {
+    if *key != 0xFF {
+        return BreakType::Proceed;
+    }
+    match length_take(decb::u8_as_usize).parse_next(input) {
+        Ok(_) => BreakType::Done,
+        // cut short: a streaming decoder waits for the rest
+        Err(_) => BreakType::Proceed,
+    }
+}
```

| `BreakType` | the decoder then |
|---|---|
| `Proceed` | decodes the item, from just after the key |
| `Skip` | reads the next key, from where the function left the input |
| `Done` | stops where the function left the input, and returns the struct |
| `Abort` | stops, and returns an error carrying its message |

```klv
k 01 l 01 v 2A      // 1. check_key: Proceed; seq = 42
k 02 l 02 v 09 2E   // 2. check_key: Proceed; temp = 2350
k FF l 00           // 3. check_key reads FF's length and value: Done
k 01 l 01 v 2B      // 4. left in the input: the next packet
```

## Run this example

```sh
cargo run --example book_control_break_on_fn
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** in `check_key`, return this in place of `BreakType::Done`:

```rust,ignore
BreakType::Abort("end of packet")
```

<details>
<summary>Answer</summary>

It prints the same line, then `unwrap` panics. The error carries the message:

```text
called `Result::unwrap()` on an `Err` value: ContextError { context: [Label("end of packet"), Label("tinyklv::ErrorKind::Aborted")], cause: None }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/control/book_control_break_on_fn.rs}}
```

</details>

**Next:** [Break on a field value](./03-decide-per-value.md)
