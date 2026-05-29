# KLV vs protobuf: benchmark analysis

A reading of *why* the numbers in [`bench.csv`](bench.csv) land where they do, grounded in the
source of each crate (`prost 0.14`, `quick-protobuf 0.8`, `rust-protobuf 3.7`, `micropb 0.6`,
`tlv_parser 0.10`, `serde_klv 0.3`) plus this repo. The timings are measured (criterion median,
`-C target-cpu=native`); the *attributions* below are evidence-based hypotheses from the code, not
profiler output — they're labelled as such.

## What is measured

Eight approaches round-trip three records — `flat` (8 primitives), `nested` (sub-packet +
`Vec<Reading>`), `native_nested` (12 common native Rust types) — across decode/encode ×
clean/framed. Four are KLV-family (`tinyklv`, hand-written `manual`, `serde_klv`, `tlv_parser`),
four are protobuf (`prost`, `quick_protobuf`, `rust_protobuf`, `micropb`). Every approach is gated
to produce the *same* decoded value before timing (`benches/suite/checks.rs`); wire formats differ
per family, so this compares "given this domain record, how fast is each library", not same-bytes.

Median ns/call (lower = faster); bold = fastest in row:

| record · op · framing | tinyklv | manual | serde_klv | tlv_parser | prost | quick_pb | rust_pb | micropb |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| flat · decode · clean   | 72.2 | 84.7 | 321.5 | 952.7 | **61.4** | 114.0 | 83.7 | 117.6 |
| flat · encode · clean   | 75.3 | 67.1 | 221.3 | 407.7 | **34.2** | 95.4 | 46.5 | 92.8 |
| nested · decode · clean | 189.0 | **158.1** | 449.2 | 1398.5 | 234.0 | 285.4 | 303.7 | 288.4 |
| nested · encode · clean | 144.7 | 129.2 | 325.7 | 637.7 | **85.5** | 141.9 | 142.7 | 135.4 |
| native · decode · clean | 281.4 | **200.3** | 579.3 | 1814.7 | 248.0 | 270.1 | 312.2 | 291.5 |
| native · encode · clean | 224.6 | 161.3 | 413.7 | 848.5 | **115.5** | 162.2 | 159.7 | 171.0 |

(Framed rows add a roughly constant per-call cost — see "Framing" below — and are in `bench.csv`.)

## The two hypotheses raised — answered from source

### 1. Field ordering: not a differentiator

Every decoder here reads a key/tag, then dispatches on it in a loop, so **all of them accept
fields in any order** — KLV and protobuf alike:

- tinyklv / `manual`: a `while` loop reading `key, len, value` then `match key` (see
  `manual/flat.rs`); tinyklv's derive does the same over winnow.
- `quick_protobuf` generated: `while !r.is_eof() { match r.next_tag(bytes) { Ok(tag) => ... } }`.
- `prost`: the runtime `Message::merge` loops `decode_key` → `merge_field(tag, ...)`.
- `rust_protobuf`: `CodedInputStream` loop over `read_tag_unpack`.
- `micropb`: `decode` loops `decode_tag` → per-field `match`.

protobuf is in fact *more* permissive than the bench exercises: by spec a decoder must also accept
**repeated/duplicated** fields (last-wins for scalars) and **skip unknown** tags. So ordering costs
neither family anything here, and it is not why protobuf is faster or slower. (KLV has the same
skip-unknown property — the `_ => {}` arm in `manual`.)

### 2. Optional / presence: a latent protobuf advantage, not triggered here

proto3 has no per-field presence for scalars: a field equal to its default (`0`, `0.0`, `""`, empty
bytes) is **omitted from the wire entirely** and re-materialised as the default on decode. So on
*sparse* data protobuf can encode/decode far less than KLV, which always emits every field. The
benchmark defeats this on purpose — `RngSample` fills every field with a random, almost-always
non-zero value — so protobuf pays full freight for all 8–12 fields and the advantage never shows.
On real-world sparse messages (many zero/empty fields) protobuf would pull ahead of these numbers;
tinyklv/KLV would not change. Message fields (`coord`, and the WKT `timestamp`/`elapsed`) *are*
presence-tracked (prost `Option`, rust-protobuf `MessageField`, micropb a `_has` hazzer), which adds
a branch and, for rust-protobuf, a heap `Box`.

## The real drivers

### Encode — it's the allocation strategy

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
the derived struct encoder allocates a small `Vec` for *each* field and concatenates them — 8–12
tiny heap allocations per record plus the copies. That overhead (not the wire format) is why
tinyklv encode (75 / 145 / 225 ns) trails prost (34 / 86 / 116 ns) and even `manual`.

`manual` is the tell that proves it's allocation, not KLV: it builds into a *single* `Vec` via
`out.extend_from_slice(&field.to_be_bytes())` — `to_be_bytes()` is a stack array, so there is **no
per-field heap allocation** (only the coord sub-buffer and the label). That alone puts `manual`
(67 / 129 / 161 ns) well ahead of tinyklv on encode. `rust_protobuf` and `micropb` also
"compute size, then write into one buffer" (`compute_size` / `get_cached_size` → `CodedOutputStream`
/ `PbEncoder`), which is why they sit between prost and tinyklv.

> Likely the single biggest tinyklv win available: have the derive encode into one
> caller-provided buffer (an `encode_into(&self, &mut Vec<u8>)`-style path) instead of returning a
> `Vec` per field. The trait docs in `src/traits/mod.rs` already flag this as desired.

### Decode — abstraction depth and whether you allocate a tree

Decode ranks roughly by how much machinery sits between the bytes and the struct:

- **`manual` fastest** (85 / 158 / 200 ns): raw slice indexing, `from_be_bytes(try_into)`, zero
  trait dispatch, zero parser framework, one `match`.
- **`prost`** (61 / 234 / 248 ns): tight hand-rolled varint + a generated `match` on the tag; on
  `flat` it even beats `manual` because varint ints are fewer bytes to touch.
- **tinyklv** (72 / 189 / 281 ns): competitive, but each field goes through a `DecodeValue`
  dispatch over winnow (checkpoints, `ErrMode`, `Result` per field) — a small constant tax that
  grows with field count (hence it's closest to `manual` on `flat`, furthest on `native`).
- **`quick_protobuf` / `micropb`** (114–290 ns): a `next_tag` loop with per-field `match`; solid,
  a notch behind prost's tuned path.
- **`rust_protobuf`** (84 / 304 / 312 ns): `CodedInputStream` + `MessageField` (heap `Box` for
  sub-messages) + `special_fields` bookkeeping — heavier on nested/native.
- **`serde_klv`** (322 / 449 / 579 ns): a serde `Deserializer`/`Visitor` driving a map-style field
  population — generic-serialization overhead the others avoid.
- **`tlv_parser` slowest by 6–10×** (953 / 1399 / 1815 ns): it first **allocates an owned BER tree**
  (`Value::TlvList(Vec<Tlv>)`, boxed nodes) for the whole record, then this bench looks each field
  up by **string path** — `find_val("21 / 09")` does a `split('/')` and walks the tree per field
  (`tlv_parser-0.10.0/src/tlv.rs:171,190`). Tree allocation + per-field string parsing is worst of
  both worlds. It is included as the "generic TLV tree" baseline, not a straw man.

### Why nested decode *flips* to KLV's favour

On `nested`, `manual` (158) and `tinyklv` (189) beat **every** protobuf crate (234–304). A protobuf
embedded message (`coord`) is a length-delimited sub-record: the decoder reads a length, then
recurses, and prost/rust-protobuf wrap it in `Option`/`Box`/`MessageField` (an allocation/branch).
A KLV sub-packet is just another key whose value is decoded in place — cheaper. Encode stays
prost's (one buffer, varint), so only nested *decode* flips.

### native_nested: tinyklv is mid-pack, and that's the point

tinyklv is neither fastest nor slowest here (281 dec / 225 enc). Two source-level reasons:

- The native types add a `DecodeValue`/`EncodeValue` dispatch *and validation* per field
  (`char::try_from`, `NonZeroU32::new`, `NaiveDate`/`NaiveTime` range checks in
  `src/traits/native.rs`) — work the raw-integer paths don't do — plus the per-field `Vec` encode
  cost above, now over 12 fields.
- prost/rust-protobuf carry `timestamp`/`elapsed` as **WKT sub-messages**
  (`google.protobuf.Timestamp`/`Duration`, see `native_wkt.proto`) — two extra length-delimited
  frames + `Option` — which is why prost's native decode (248) is much closer to micropb's raw-i64
  path (291) than its commanding `flat` lead would suggest. micropb/quick read those as plain
  `int64`/`uint64`.

The native record's purpose is **expressiveness, not raw speed**: tinyklv's `native.rs` is a 4-line
passthrough because the fields *are* `DateTime<Utc>`/`Ipv4Addr`/`NonZeroU32`; the protobuf crates
need a whole generated struct plus a `TryFrom` conversion (a generated field can never *be* a native
type), and the KLV libraries hand-convert field by field. The bars are competitive; the line count
is not.

## Measurement caveats (read before quoting a number)

- **Encode includes the map.** For protobuf, `encode` measures "build the generated message from
  the domain value, then serialize" (documented in each `*/mod.rs`); the field-by-field map is part
  of the cost, by design. So is tinyklv's struct→bytes.
- **WKT vs raw is not apples-to-apples wire.** prost/rust-protobuf native use sub-message
  timestamp/duration; micropb/quick use raw integers — a deliberate honesty choice (each crate uses
  the native conversions it actually has), not a controlled-wire comparison.
- **Framing is ~constant.** Framed rows add the shared sentinel-seek (`framing::seek`,
  `windows(2).position`) for the non-tinyklv approaches, and tinyklv's native `decode_frame`
  (a `memchr` seek) for itself — roughly +15–50 ns regardless of approach; it does not change the
  ranking.
- **These are micro-benchmarks** on small records with warm caches and `target-cpu=native`.
  Allocation cost dominates precisely because the per-call work is tiny; on larger payloads the
  varint/byte-copy ratios shift.

## Takeaways

1. **prost is the encode champion** because of a single sized allocation + varint, not because of
   the wire format per se. tinyklv's biggest opportunity is to stop returning a `Vec` per field.
2. **Field order and (in this bench) optionality don't explain the gaps** — every decoder is
   order-tolerant; protobuf's zero-field omission is a real advantage that this dataset deliberately
   doesn't exercise.
3. **KLV wins nested decode** because sub-packets are cheaper than length-delimited embedded
   messages with `Option`/`Box`.
4. **`manual` is the floor, `tlv_parser` the ceiling** — the spread is allocation + abstraction
   depth (tree-build + string-path lookup at the slow end).
5. **For native Rust types, tinyklv trades a little speed for a lot less code** — the whole reason
   the `native_nested` record exists.

Regenerate the data with `benches/scripts/charts.sh -f` (writes `bench.jpg`); the medians here come
from `target/criterion/*/new/estimates.json`.
