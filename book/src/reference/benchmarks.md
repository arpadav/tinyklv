# Benchmarks

The benchmark suite compares tinyklv against hand-written KLV, `serde_klv`,
`tlv_parser`, and protobuf stacks (`prost`, `quick_protobuf`, `rust_protobuf`,
`micropb`) across simple, compound, rich/native, value-only, framed, and streamed
records.

![tinyklv vs KLV/TLV crates](https://github.com/arpadav/tinyklv/blob/main/benches/bench_klv.jpg?raw=true)

The KLV/TLV comparison shows tinyklv close to the manual implementation while
remaining derive-based. The biggest wins come from direct decode into user
structs, fixed-width codec specialization, and generated dispatch without an
owned intermediate tree.

![tinyklv vs protobuf stacks](https://github.com/arpadav/tinyklv/blob/main/benches/bench_proto.jpg?raw=true)

The protobuf comparison is not a same-wire-format claim: protobuf uses varints
and generated message types, while tinyklv uses explicit KLV key/length/value
triples. The useful takeaway is practical throughput: on these records, tinyklv
is in protobuf territory and often faster while preserving protocol-agnostic KLV
framing.

Benchmark machine:

| Component | Value |
|-----------|-------|
| OS | Ubuntu 24.04.4 LTS |
| Kernel | Linux `6.17.0-35-generic` |
| CPU | AMD Ryzen 9 9900X |
| Host | B850 AI TOP -CF-WCP-ADO |

The raw benchmark data and chart scripts live in `benches/`.
