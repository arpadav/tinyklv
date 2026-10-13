# Tutorial 14 - Break conditions

The derive-generated decode loop reads `(key, len)`, matches the key,
decodes the value, and moves on. Sometimes you need more:

- **Skip** - discard a reserved key's frame without erroring.
- **Done** - stop early when a terminator key appears, returning whatever
  was accumulated.
- **Abort** - stop and propagate an error, e.g. on a tamper-detection key.

`tinyklv` models all four outcomes with `BreakType`:

```rust,ignore
pub enum BreakType {
    Proceed,             // run the normal decode step for this key
    Skip,                // continue the loop past what the fn read
    Done,                // stop looping, return accumulated fields
    Abort(&'static str), // stop looping, return Err labelled with the message
}
```

You wire a break condition into a derived decoder with the container
attribute `#[klv(break_on = ..)]`. It accepts either form:

- a **key literal** - `break_on = 0xFF` means "stop the loop (`Done`) when a
  decoded key equals `0xFF`". The terminator shorthand.
- a **function path** - `break_on = classify`, where you write
  `fn classify(key: &u8, input: &mut &[u8]) -> BreakType` and return any
  outcome. It runs right after every key and may read on: `Proceed` rewinds
  to just after the key, the others keep the input where the function left it.

Both forms feed the **same** loop behind `decode_value`, `decode_frame` and
the streaming `decoder()`, so a break condition behaves alike one-shot or
across fragmented reads.

The example wires up a heartbeat where the transmitter emits a reserved key
(`0xFE`, ignored today but meaningful to future firmware) and a terminator
key (`0xFF`, stop decoding here). `classify` reads the reserved frame's
length and value before `Skip`, and returns `Proceed` on a short read, so the
frame reads as cut and streaming waits.

Run this example: `cargo run --example book_14_break_condition`

```rust,no_run
{{#include ../../../../examples/book/advanced/book_14_break_condition.rs}}
```

- `break_on = <literal>` stops on a key match; `break_on = <fn>` runs your
  classifier for full control (including `Skip` and `Abort`).
- The same derive covers encode and decode - no hand-written mirror needed.

For loop control beyond what `break_on` expresses, you can still drop the
derive on the decode side and hand-write a `DecodeValue` impl with the logic
inline (see `examples/packets/14_break_condition_custom.rs`).

**Next:** [15 - Streaming partial packets](./15-streaming-decode.md)
