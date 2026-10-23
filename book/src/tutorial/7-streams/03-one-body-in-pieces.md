# Partial decoding

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: the whole body at once

`decode_value` reads a body that is all in the input.

## One body in pieces, no sentinel

Say a transport hands you one body in pieces, with no sentinel for a [streaming decoder](./02-a-live-stream.md) to find. `decode_partial` decodes what has arrived, and returns a `Packet`:

| `Packet` | means |
|---|---|
| `Ready` | the decoder stopped by itself, as at a `break_on`: it holds the struct |
| `NeedMore` | the input ran out: it holds the fields decoded so far |

`resume_partial` continues from a `NeedMore` with more bytes:

```rust,ignore
{{#include ../../../../examples/book/streams/book_streams_decode_partial.rs:pieces}}
```

```klv
k 01 l 01 v 2A      // 1. decode_partial: seq = 42
k 02 l 02           // 2. temp is cut: NeedMore; the input stays at its key
k 02 l 02 v 09 2E   // 3. resume_partial on the rest: temp = 2350
                    // 4. the input ends: NeedMore
                    // 5. finalize: seq and temp are in: the Sensor
```

> [!NOTE]
> **The end of the input is not the end of the body.** `decode_partial` cannot tell them apart, so it returns `NeedMore` even when every field is in. Once you know the body is whole, call `finalize`. It returns an error if a field the struct needs is still missing.

## Run this example

```sh
cargo run --example book_streams_decode_partial
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** in `split_at`, use 7 in place of 5, so the first piece is the whole body.

<details>
<summary>Answer</summary>

It prints the same line, then the first `assert_eq!` fails. `decode_partial` still returns `NeedMore`, and leaves the input empty, not at temp's key.

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/streams/book_streams_decode_partial.rs:all}}
```

</details>

**Next:** [Tokio](./04-tokio.md)
