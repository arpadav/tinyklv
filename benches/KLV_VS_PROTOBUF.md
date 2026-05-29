# KLV vs protobuf: benchmark analysis

A reading of *why* the numbers in [`bench.csv`](bench.csv) land where they do, grounded in the
source of each crate (`prost 0.14`, `quick-protobuf 0.8`, `rust-protobuf 3.7`, `micropb 0.6`,
`tlv_parser 0.10`, `serde_klv 0.3`) plus this repo. The timings are measured (criterion median,
`-C target-cpu=native`); the *attributions* below are evidence-based hypotheses from the code, not
profiler output - they're labelled as such.

## What is measured

Eight approaches round-trip three records - `simple` (8 primitives), `compound` (sub-packet +
`Vec<Reading>`), `rich` (12 common native Rust types) - across decode/encode ×
clean/framed. Four are KLV-family (`tinyklv`, hand-written `manual`, `serde_klv`, `tlv_parser`),
four are protobuf (`prost`, `quick_protobuf`, `rust_protobuf`, `micropb`). Every approach is gated
to produce the *same* decoded value before timing (`benches/suite/checks.rs`); wire formats differ
per family, so this compares "given this domain record, how fast is each library", not same-bytes.

Median ns/call (lower = faster); bold = fastest in row:

| record · op · framing | tinyklv | manual | serde_klv | tlv_parser | prost | quick_pb | rust_pb | micropb |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| simple · decode · clean   | 19.3 | **10.1** | 74.8  | 525.2  | 23.3 | 26.9 | 29.4 | 34.6 |
| simple · encode · clean   | 62.6 | 59.0 | 219.1 | 417.7  | **21.9** | 70.3 | 32.7 | 78.3 |
| compound · decode · clean | 51.3 | **19.5** | 110.4 | 739.0  | 122.5 | 122.5 | 152.2 | 133.2 |
| compound · encode · clean | 125.4 | 109.4 | 306.1 | 635.5 | **45.1** | 101.7 | 131.9 | 100.5 |
| rich · decode · clean     | 43.0 | **31.4** | 153.9 | 931.2  | 102.9 | 78.4 | 118.9 | 87.6 |
| rich · encode · clean     | 164.5 | 107.8 | 348.9 | 794.5 | **60.4** | 105.2 | 116.9 | 117.9 |

(Framed rows add a roughly constant per-call cost - see "Framing" below - and are in `bench.csv`.)

## The two hypotheses raised - answered from source

### 1. Field ordering: not a differentiator

Every decoder here reads a key/tag, then dispatches on it in a loop, so **all of them accept
fields in any order** - KLV and protobuf alike:

- tinyklv / `manual`: a `while` loop reading `key, len, value` then `match key` (see
  `manual/simple.rs`); tinyklv's derive does the same over winnow.
- `quick_protobuf` generated: `while !r.is_eof() { match r.next_tag(bytes) { Ok(tag) => ... } }`.
- `prost`: the runtime `Message::merge` loops `decode_key` -> `merge_field(tag, ...)`.
- `rust_protobuf`: `CodedInputStream` loop over `read_tag_unpack`.
- `micropb`: `decode` loops `decode_tag` -> per-field `match`.

protobuf is in fact *more* permissive than the bench exercises: by spec a decoder must also accept
**repeated/duplicated** fields (last-wins for scalars) and **skip unknown** tags. So ordering costs
neither family anything here, and it is not why protobuf is faster or slower. (KLV has the same
skip-unknown property - the `_ => {}` arm in `manual`.)

### 2. Optional / presence: a latent protobuf advantage, not triggered here

proto3 has no per-field presence for scalars: a field equal to its default (`0`, `0.0`, `""`, empty
bytes) is **omitted from the wire entirely** and re-materialised as the default on decode. So on
*sparse* data protobuf can encode/decode far less than KLV, which always emits every field. The
benchmark defeats this on purpose - `RngSample` fills every field with a random, almost-always
non-zero value - so protobuf pays full freight for all 8–12 fields and the advantage never shows.
On real-world sparse messages (many zero/empty fields) protobuf would pull ahead of these numbers;
tinyklv/KLV would not change. Message fields (`coord`, and the WKT `timestamp`/`elapsed`) *are*
presence-tracked (prost `Option`, rust-protobuf `MessageField`, micropb a `_has` hazzer), which adds
a branch and, for rust-protobuf, a heap `Box`.

## The real drivers

### Encode - it's the allocation strategy

This is the clearest signal in the data: **`prost` wins every encode row**, and the reason is in
`prost-0.14.3/src/message.rs:61`:

```rust
fn encode_to_vec(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(self.encoded_len()); // ONE exactly-sized allocation
    self.encode_raw(&mut buf);                            // contiguous varint writes
    buf
}
```

One `encoded_len()` pass, one allocation, one linear write of compact varints. Contrast tinyklv,
whose `EncodeValue::encode_value(&self) -> Vec<u8>` returns an **owned `Vec` per field**
(`src/codecs/binary/enc.rs`: `pub fn be_u64(input) -> Vec<u8> { input.to_be_bytes().to_vec() }`):
the derived struct encoder allocates a small `Vec` for *each* field and concatenates them - 8–12
tiny heap allocations per record plus the copies. That overhead (not the wire format) is why
tinyklv encode (63 / 125 / 165 ns) trails prost (22 / 45 / 60 ns) and even `manual`.

`manual` is the tell that proves it's allocation, not KLV: it builds into a *single* `Vec` via
`out.extend_from_slice(&field.to_be_bytes())` - `to_be_bytes()` is a stack array, so there is **no
per-field heap allocation** (only the coord sub-buffer and the label). That alone puts `manual`
(59 / 109 / 108 ns) well ahead of tinyklv on encode. `rust_protobuf` and `micropb` also
"compute size, then write into one buffer" (`compute_size` / `get_cached_size` -> `CodedOutputStream`
/ `PbEncoder`), which is why they sit between prost and tinyklv.

> Likely the single biggest tinyklv win available: have the derive encode into one
> caller-provided buffer (an `encode_into(&self, &mut Vec<u8>)`-style path) instead of returning a
> `Vec` per field. The trait docs in `src/traits/mod.rs` already flag this as desired.

### Decode - abstraction depth and whether you allocate a tree

Decode ranks roughly by how much machinery sits between the bytes and the struct:

- **`manual` fastest** (10 / 20 / 31 ns): raw slice indexing, `from_be_bytes(try_into)`, zero
  trait dispatch, zero parser framework, one `match`. The floor by construction.
- **tinyklv second - and ahead of every protobuf crate on decode** (19 / 51 / 43 ns): each field
  still goes through a `DecodeValue` over winnow, but the leaf int decoders are now a single sized
  read + `from_be_bytes` (one load + bswap), the same shape `manual` uses, so the old per-field
  shift/add tax is gone. tinyklv beats `prost` on all three records.
- **`prost`** (23 / 123 / 103 ns): tight hand-rolled varint + a generated `match` on the tag.
  Fastest *protobuf* on `simple`, but its embedded-message handling makes it the slowest readable
  option on `compound` (see below).
- **`quick_protobuf` / `micropb`** (27–133 ns): a `next_tag` loop with per-field `match`; a notch
  behind prost on `simple`, ahead of it on `compound`/`rich` (no `Box`/WKT overhead).
- **`rust_protobuf`** (29 / 152 / 119 ns): `CodedInputStream` + `MessageField` (heap `Box` for
  sub-messages) + `special_fields` bookkeeping - heaviest protobuf on compound/rich.
- **`serde_klv`** (75 / 110 / 154 ns): a serde `Deserializer`/`Visitor` driving a map-style field
  population - generic-serialization overhead the others avoid.
- **`tlv_parser` slowest by ~15–50×** (525 / 739 / 931 ns): it first **allocates an owned BER tree**
  (`Value::TlvList(Vec<Tlv>)`, boxed nodes) for the whole record, then this bench looks each field
  up by **string path** - `find_val("21 / 09")` does a `split('/')` and walks the tree per field
  (`tlv_parser-0.10.0/src/tlv.rs:171,190`). Tree allocation + per-field string parsing is worst of
  both worlds. It is included as the "generic TLV tree" baseline, not a straw man.

### Why compound decode punishes protobuf

On `compound`, `manual` (20) and `tinyklv` (51) beat **every** protobuf crate (123–152) by 2–3×. A
protobuf embedded message (`coord`) is a length-delimited sub-record: the decoder reads a length,
then recurses, and prost/rust-protobuf wrap it in `Option`/`Box`/`MessageField` (an allocation/
branch). A KLV sub-packet is just another key whose value is decoded in place - cheaper. This is
the record where prost's decode jumps most (23 -> 123 ns) while tinyklv's grows far less (19 -> 51).
Encode stays prost's (one buffer, varint), so the protobuf penalty is decode-only.

### rich: strong decode, mid-pack encode

On `rich` *decode* tinyklv (43) is second only to `manual` (31) and beats every protobuf crate
(prost 103, quick 78, micropb 88). On *encode* it is mid-pack (165 vs prost 60, manual 108) - the
per-field `Vec` allocation, now over 12 fields, plus native validation. Two source-level reasons it
holds up on decode despite doing more work per field:

- The native types add a `DecodeValue`/`EncodeValue` dispatch *and validation* per field
  (`char::try_from`, `NonZeroU32::new`, `NaiveDate`/`NaiveTime` range checks in
  `src/traits/native.rs`) - work the raw-integer paths don't do.
- prost/rust-protobuf carry `timestamp`/`elapsed` as **WKT sub-messages**
  (`google.protobuf.Timestamp`/`Duration`, the WKT `Rich` schema) - two extra length-delimited
  frames + `Option` - which is why prost's `rich` decode (103) sits well behind tinyklv (43) here.
  micropb/quick read those as plain `int64`/`uint64`.

The record's purpose is **expressiveness**: tinyklv's `native.rs` is a 4-line passthrough because
the fields *are* `DateTime<Utc>`/`Ipv4Addr`/`NonZeroU32`; the protobuf crates need a whole generated
struct plus a `TryFrom` conversion (a generated field can never *be* a native type), and the KLV
libraries hand-convert field by field. With the decoder now fast too, that expressiveness no longer
costs decode speed.

## Measurement caveats (read before quoting a number)

- **Encode includes the map.** For protobuf, `encode` measures "build the generated message from
  the domain value, then serialize" (documented in each `*/mod.rs`); the field-by-field map is part
  of the cost, by design. So is tinyklv's struct->bytes.
- **WKT vs raw is not apples-to-apples wire.** prost/rust-protobuf native use sub-message
  timestamp/duration; micropb/quick use raw integers - a deliberate honesty choice (each crate uses
  the native conversions it actually has), not a controlled-wire comparison.
- **Framing is ~constant and small.** Framed rows add the shared sentinel-seek (`framing::seek`,
  `windows(2).position`) for the non-tinyklv approaches, and tinyklv's native `decode_frame`
  (an inlined `memchr` seek for short sentinels) for itself - roughly +1–9 ns regardless of
  approach; it does not change the ranking.
- **These are micro-benchmarks** on small records with warm caches and `target-cpu=native`.
  Allocation cost dominates precisely because the per-call work is tiny; on larger payloads the
  varint/byte-copy ratios shift.

## Takeaways

1. **prost is the encode champion** because of a single sized allocation + varint, not because of
   the wire format per se. tinyklv's biggest opportunity is to stop returning a `Vec` per field.
2. **Field order and (in this bench) optionality don't explain the gaps** - every decoder is
   order-tolerant; protobuf's zero-field omission is a real advantage that this dataset deliberately
   doesn't exercise.
3. **KLV wins nested decode** because sub-packets are cheaper than length-delimited embedded
   messages with `Option`/`Box`.
4. **`manual` is the floor, `tlv_parser` the ceiling** - the spread is allocation + abstraction
   depth (tree-build + string-path lookup at the slow end).
5. **For native Rust types, tinyklv trades a little speed for a lot less code** - the whole reason
   the `native_nested` record exists.

Regenerate the data with `benches/scripts/gencharts.sh -f` (writes `bench.jpg`); the medians here come
from `target/criterion/*/new/estimates.json`.
