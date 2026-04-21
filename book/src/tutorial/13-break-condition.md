# Tutorial 13 - Break conditions

The derive-generated decode loop reads `(key, len)`, matches the key,
decodes the value, and moves on. Occasionally you need something the
standard loop doesn't model:

- **Skip** - consume and discard a reserved key's value without erroring.
- **Done** - stop early when a terminator key appears, returning whatever
  was accumulated.
- **Abort** - stop and propagate an error, e.g. on a tamper-detection key.

Tinyklv models all four outcomes with `BreakConditionType`:

```rust,ignore
pub enum BreakConditionType {
    Proceed,         // run the normal decode step for this key
    Skip,            // consume value bytes, continue the loop
    Done,            // stop looping, return accumulated fields
    Abort(Error),    // stop looping, return Err(Error)
}
```

The `BreakCondition` trait has a blanket impl that always returns `Proceed`,
so derived types get the standard loop for free. To override it, **you
cannot derive `Klv` for the decoder side** - the blanket impl wins. Instead,
drop the derive on the decoder type and write a manual `DecodeValue` impl
that embeds the break-condition match inside the loop body. The encode side
can keep `#[derive(Klv)]` on a mirror struct that only produces bytes.

The example wires up a heartbeat where the transmitter emits a reserved key
(`0xFE`, ignored today but meaningful to future firmware) and a terminator
key (`0xFF`, stop decoding here). A `classify(key)` helper keeps the outcome
table separate from the loop body, so the loop itself reads top to bottom.

```rust,no_run
{{#include ../../../examples/book_13_break_condition.rs}}
```

- `BreakConditionType` enumerates the four loop outcomes.
- The blanket impl blocks override on derived types - manual `DecodeValue` is required.
- Encode side can still derive; a mirror struct keeps the byte layout honest.

**Next:** [14 - Sentinel seeking in pipelines](./14-sentinel-seeking.md)
