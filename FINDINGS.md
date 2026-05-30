# Findings

Open items, and dead ends so agents don't re-walk them. Each "tried" note is **the current
approach + what was tried and rejected (and why)** - do not "re-fix" these without new data.

## To investigate (notes from user)

1. i want to understand more on the sizes of these streams, and how they differ. sometimes the streams
are smaller than protobufs, sometimes they are larger. to maximize throughput, i want to minimize my stream
size always. where is this going wrong and how to fix?
2. is the lazylock memchar (or whatever its called) that helps with sentinel seeking truly required? i know it was LEAGUES faster than std lib, but that was 2-3 years ago i benchmarked it. id like to a. benchmark again b. what is penalty of it being in a lazylock? can in be trait impl'd? and if no sentinel defined either make a blank one or an optional one? i believe it couldnt be defined on the impl cus it wasnt a const.. but i just want to investigate this further

## General todos

1. in readme, i want to add a chart with check / X / - between tinyklv and the rest of the crates
in the benchmark, including benchmark results. for example, allows for partial packet recovery, optional
types, built in/optimized streaming, etc. think of some metrics. see the protobuf generated.rs's and see their comments on truncation
and whatnot, and also read their source code (for the protobuf and the other tlv/klv crates) to think 
of some metrics and see what crate does what. i dont want to lie, so if a crate does something that 
tinyklv does not, then document it.
2. based off benchmarks and this table, i want to update the book with wording, using technical writer
to help see where wording can be improved to indicate that tinyklv is fast, and lightweight
3. use technical writer to also re-write the README in the summary chart and the speed section. the summary
will just compare tinyklv to other tlv/klv crates + manual, but then in the speed we need to talk about 
that AND THEN say "its also so fast its better than protobuf crates" and then show that chart. talk minimally
about HOW the benchmarks were run, and offload that information to benches/README.md (write it)
* i want to modify the new encoder fn signature to input mut output FIRST/PRIOR to input T, this follows ancient fortran matlab c type stuff where mut outputs come first, then input args

## Open / deferred

- **Deeper in-place `decode_value`.** The one-shot path fills a `Partial` (`Option<T>`-per-field)
  then `finalize`s. Writing straight into locals seeded to defaults (prost's `merge_field` shape, no
  second pass) would shave a little more, but `compound` is already ~1.1x `manual`, so low value, and
  it touches derive output shared with the streaming path. Not started. (`decode_partial_gen.rs`)
- **`break_on = <literal>` requires an integer-comparable key type** (lowers to `if key == <lit>`);
  exotic key types must use the `break_on = <fn>` form. Worth a docs line. (`decode_partial_gen.rs`)
- **Encode `encoded_len()` size-pass (prost's model) - not done, the eventual ceiling.** Encode
  currently uses direct fixed-width writes + length backpatch + a reused scratch (the `FieldEncodeStrategy`),
  with a cheap compile-time capacity hint. prost instead does a pure `encoded_len()` pass per
  field/leaf, then **one** `Vec::with_capacity(total)` and a single linear write with all lengths
  known up front. We deliberately did NOT do this - direct-write + backpatch already reach prost's
  allocation model at the benchmarked sizes, and it's the heaviest lift (every leaf needs an
  `encoded_len`). Revisit only if profiling shows capacity reallocs dominating on **large** payloads.
  (`impl/src/expand/encode_impl.rs`)

## Tried and rejected / current approach (don't re-walk)

- **`Vec<T>::decode_value` pre-sizes** with a memory-bounded `with_capacity` hint. Starting from an
  empty `Vec` (cap 0->4->8…) was ~2.7x slower on small runs (a probe; the *entire* gap vs a hand loop).
  Do not revert to `Vec::new`. (`src/traits/dec/mod.rs`)
- **The `Vec<T>` per-element `checkpoint`/`eof` loop stays.** The reset is load-bearing for the
  zero-consumed-rewind contract; the measured cost was the missing capacity, not the loop. Removing
  it is high risk, ~zero reward. (`src/traits/dec/mod.rs`)
- **No `*_from_slice` / fixed-width bounds-check elision.** A probe showed `simple` decode (all
  fixed-width, max exposure) already ties `manual` - LLVM elides the inner `take(N)` after the outer
  `take(len)`. A `*_from_slice` leaf family buys ~0. (`src/codecs/binary/dec.rs`)
- **Encode does not chase `prost` on `compound`/`rich`.** The remaining gap is varint vs fixed-width
  wire density - inherent to KLV's wire format, not allocation. Matching it would mean changing the
  bytes on the wire.
- **Streaming fresh-mode decodes complete framed bodies with the one-shot `decode_value`** (the
  `Partial`/resume machinery is only for genuinely cross-buffer packets in resume mode). The residual
  gap to `manual` on `*_decode_streamed` is the per-packet `seek_sentinel` (memchr + length parse) +
  `BufCursor` bookkeeping; a separate lever if streamed throughput matters. (`src/decoder/iter.rs`)
- **Bench `Reading::pack` allocates a temp `Vec`.** Left as-is: ranking-neutral (the `manual`
  approach calls the same fn). Removing it from both is optional bench hygiene only.
  (`benches/suite/records/shared.rs`)

## Benchmark / cross-crate notes (don't re-derive; feeds the README todos)

The full per-crate "why it's fast/slow" analysis (prost/quick-protobuf/rust-protobuf/micropb/
tlv_parser/serde_klv source reads) is backed up at `/mnt/arpadnas/vault/KLV_VS_PROTOBUF.md` - pull
from it for README todos 1 & 3. The durable, decision-relevant nuggets:

- **Field ordering is not a differentiator.** Every decoder here (KLV and protobuf) reads a key/tag
  then `match`es in a loop, so all are order-tolerant and skip-unknown. Don't "optimize" ordering.
- **proto3 zero-field omission is a real protobuf edge on SPARSE data - and bears on stream size
  (investigate-note #1).** A proto3 scalar equal to its default (`0`/`""`/empty) is omitted from the
  wire and re-defaulted on decode; tinyklv/KLV always emit every field. The benchmark deliberately
  defeats this (`RngSample` fills every field non-zero), so it never shows here - but on real sparse
  messages protobuf streams smaller. This is the main reason tinyklv streams are sometimes larger
  than protobuf; closing it would require optional-field omission on the wire (a format choice).
- **KLV wins nested decode** (compound: tinyklv/manual beat every protobuf 2-3x): a KLV sub-packet is
  a key whose value decodes in place, vs a protobuf embedded message = length-delimited sub-record
  wrapped in `Option`/`Box`/`MessageField`. Do not "fix" nested decode - it's already the win.
- **`manual` is the floor, `tlv_parser` the ceiling.** The spread is allocation + abstraction depth;
  `tlv_parser` is ~15-50x slow because it builds an owned BER tree then looks up fields by string
  path. tinyklv sits just above `manual` and below everything else.
- **Reading the bench numbers:** encode timings include the domain-value->message map (protobuf) /
  struct->bytes (tinyklv), by design; protobuf native `timestamp`/`elapsed` use WKT sub-messages while
  micropb/quick use raw ints, so encode is not a same-wire comparison; framing adds a small ~constant
  per call; these are micro-benchmarks where allocation dominates because per-call work is tiny.
