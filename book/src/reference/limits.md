# Limits

tinyklv decodes what the bytes say, and does not repair them. These are the cases where that can surprise you, and what to do about each.

## Malformed input

### A length that never arrives

A packet length that its decoder accepts, but far past anything your protocol sends, makes the `Decoder` wait for that many bytes: a length of 2^31 waits for 2 GiB. Reject such a length in the length decoder, tagged `ErrorKind::MalformedValue`. The `Decoder` then reports that packet once, and moves on:

```rust,ignore
{{#include ../../../examples/book/ref/book_ref_limits.rs:packet_len}}
```

Or cap the bytes you buffer while no packet comes out, and call `clear`.

### A length that fails to read

With `key` and `len`, a length its decoder rejects ends the decode, as if the input were cut there. The fields before it are returned, and the rest stays in the input, with no error. With BER lengths:

```klv
k 01 l 01 v 0A     // 1. a = 10
k 02 l 80          // 2. 80 is no BER length: decoding ends, and returns a = 10
k 03 l 01 v 0C     // 3. not read: from key 02 on, the bytes stay in the input
```

Check that the input is empty after decoding. With [`unknown`](./container-attributes.md#unknown) on the struct, such a length is an error instead, `ErrorKind::MalformedValue`.

### What `strict` does not check

`strict` checks each value that has a length, not the input as a whole. It lets these through:

| case | what you get |
|---|---|
| bytes after the struct | they stay in the input |
| an item the end of a packet's body cuts short | the field is left unset |
| a value of `size(implicit, exact = N)` its decoder rejects | the field is left unset |
| a `Vec` value whose last element is cut short | the `Vec` without that element |
| under `keylen`, a kept unknown item its decoder reads only in part | the rest of its value is dropped |

Check that the input is empty after decoding, and count the elements of a `Vec` in your own type.

### Non-canonical BER

The BER decoders accept forms longer than they need to be: padded BER-OID groups and a long-form length for a short value. A first length byte of FF, which X.690 reserves, is rejected. The encoders write only the shortest form. To reject the longer forms, write your own decoder.

```klv
v 80 80 05    // ber_oid reads 5
l 81 05       // ber_length reads 5
```

### An item that reads nothing

With a separate key and length codec, a key decoder that only peeks at its key, beside an `implicit` value decoder that reads nothing, reads the same item forever: the decode never returns. Under `keylen`, the same item is an error, `ErrorKind::NoProgress`. A key decoder must consume its key, and an `implicit` decoder its value.

## Untrusted input

### Deep nesting

A struct that holds itself, such as through an `Option<Box<T>>`, decodes each level by recursion. Deep enough nesting overflows the stack, and the process aborts. In a release build, a one-field struct overflows a 2 MiB thread, the default for a spawned thread, at about 14,500 levels: some 58 KB of input. Cap the size of untrusted packets, or decode on a thread with a larger stack.

### Many unknown items

A [`#[klv(unknown)]`](./field-attributes.md#unknown) field keeps every unknown item a packet holds, however many. To bound them, give the field a type of your own that drops items past a limit.

### Lengths your decoder trusts

tinyklv's codecs allocate only for bytes that are there, and a `Vec` value sets aside at most 4 KiB before it reads. A decoder of yours may trust a length instead: `Vec::with_capacity` with a length of 2^62 aborts the process. Size a buffer from the bytes your decoder is handed, not from a length it reads.

### Lengths past `usize`

The `_as_usize` length decoders wrap a length too large for a `usize`. A `u128` length of 2^64 + 5 reads as 5, and so does a `u64` length of 2^32 + 5 on a 32-bit machine. For untrusted input, write a length decoder that rejects a length it cannot hold.

### Text that grows when written back

`to_string_utf8` reads each invalid byte as U+FFFD, which is 3 bytes in UTF-8. A relay that decodes text with it, then encodes it again, can grow a value past what its length can say; [`len`](./container-attributes.md#len) lists what then happens. Relay text with `to_string_utf8_strict`, which rejects invalid bytes.

```klv
v FF FE       // to_string_utf8 reads 2 U+FFFD: 6 bytes when written back
```

## Streams

### `drain_frames` stops at a bad packet

`drain_frames` returns the packets before the first one that fails, cut short or malformed, and does not return the error. The input is left just after the last packet it returns, so any packet after the bad one is not read. Call `decode_frame` there: it returns the error, and moves the input past the bad packet.

### `Decoder::next` stops at a bad packet

At a malformed packet, `next` drops the packet's bytes and returns `None`. A `while let` loop over `next` then stops, though good packets may be buffered behind it. So does `iter`. Call `next` again to go on, or use `next_fresh`, which returns the error.

### A `Vec` element cut short

`Vec<T>::decode_partial` keeps an element the input cuts short, once the element's required fields are in, and the rest of it starts the next read:

```klv
k 01 l 01 v 07    // a = 7
k 02              // cut short: the Vec holds the element, and 02 is left
```

Give each element a sentinel and use the `Decoder`, or decode the `Vec` once all of it has arrived.

### A key literal inside a `Vec`

A `break_on` key literal leaves the end item's length and value in the input. In a `Vec` of such structs, the next element starts there, and reads them as an item:

```klv
k 01 l 01 v 07    // 1st element: temp = 7
k FF              // key FF: the 1st element ends
u 00 01 01        // the 2nd reads key 00, length 1: no field, skipped
u 08 FF 00        // key 08, length 255: cut short; the 2nd ends, temp None
```

Read the end item in a function instead, before it returns `Done`, as [`break_on`](./container-attributes.md#a-function) shows.

### `Proceed` on a short read, under `keylen`

Under `keylen`, a break function may return `Proceed` when it has too few bytes to decide. Without `unknown`, an unknown `Inline` key then ends the decode, as if the packet were whole: the streaming decode returns it ready, without the fields after it. With [`unknown`](./container-attributes.md#unknown) on the struct, the decode waits for more bytes instead.

## Errors and names

### The kind of a nested error

`ErrorKind::of` returns the outermost kind. Under `strict`, a nested struct missing a field is `MalformedValue`, as is a UTF-16 value of an odd length. Their inner kinds, `MissingRequired` and `WrongLength`, are not the ones read back.

### Names the derive adds

The derive adds a type, `<Name>PartialPacket`, beside each struct it decodes, and a method, `decoder()`, to one with a sentinel. An item of your own with either name is a compile error: rename it.

## Run this example

```sh
cargo run --example book_ref_limits
```

<details>
<summary>Full example: the limits above</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_limits.rs:all}}
```

</details>
