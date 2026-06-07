# Tutorial 14 - Break conditions

The derive-generated decode loop reads `(key, len)`, matches the key,
decodes the value, and moves on. Occasionally you need something the
standard loop doesn't model:

- **Skip** - consume and discard a reserved key's value without erroring.
- **Done** - stop early when a terminator key appears, returning whatever
  was accumulated.
- **Abort** - stop and propagate an error, e.g. on a tamper-detection key.

`tinyklv` models all four outcomes with `BreakType`:

```rust,ignore
pub enum BreakType {
    Proceed,             // run the normal decode step for this key
    Skip,                // consume value bytes, continue the loop
    Done,                // stop looping, return accumulated fields
    Abort(&'static str), // stop looping, return Err labelled with the message
}
```

You wire a break condition into a derived decoder with the container
attribute `#[klv(break_on = ..)]`. It accepts either form:

- a **key literal** - `break_on = 0xFF` means "stop the loop (`Done`) when a
  decoded key equals `0xFF`". The terminator shorthand.
- a **function path** - `break_on = classify`, where you write
  `fn classify(key, len) -> BreakType` and return any outcome. The function is
  consulted after every `(key, len)` is read, before the field is dispatched.

Both forms feed the **same** generated loop used by `decode_value`,
`decode_frame`, and the streaming `decoder()` - so a break condition behaves
identically whether you decode one-shot or across fragmented reads.

The example wires up a heartbeat where the transmitter emits a reserved key
(`0xFE`, ignored today but meaningful to future firmware) and a terminator
key (`0xFF`, stop decoding here). A `classify(key, len)` function keeps the
outcome table separate from the struct, and the single derive produces both
the encoder and the break-aware decoder.

Run this example: `cargo run --example book_14_break_condition`

```rust,no_run
{{#include ../../../../examples/book_14_break_condition.rs}}
```

- `BreakType` enumerates the four loop outcomes.
- `break_on = <literal>` stops on a key match; `break_on = <fn>` runs your
  classifier for full control (including `Skip` and `Abort`).
- The same derive covers encode and decode - no hand-written mirror needed.

For loop control beyond what `break_on` expresses, you can still drop the
derive on the decode side and hand-write a `DecodeValue` impl with the logic
inline (see `examples/14_break_condition_custom.rs`).

**Next:** [15 - Streaming partial packets](./15-streaming-decode.md)
