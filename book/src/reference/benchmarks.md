# Benchmarks

tinyklv against a hand-written KLV codec and other crates, in nanoseconds per byte: lower is better.

| compared with | in the charts |
|---|---|
| KLV / TLV | manual (a hand-written codec), serde_klv, tlv_parser |
| protobuf | prost, quick_protobuf, rust_protobuf (the protobuf crate), micropb |

Each row of a chart names a record, a direction (decode or encode) and a framing. The records:

| record | holds |
|---|---|
| simple | eight numbers |
| compound | numbers, a nested position packet, and a list of readings |
| rich | native types, such as a date, a duration and an IP address |

The framings:

| framing | one call |
|---|---|
| value | decodes or encodes one body |
| frame | decodes or encodes one packet with a sentinel, after junk |
| streamed | decodes 64 such packets through the streaming decoder |

## Against KLV and TLV crates

![tinyklv vs KLV/TLV crates](https://github.com/arpadav/tinyklv/blob/main/benches/bench_klv.jpg?raw=true)

- decode: close to manual, while still a derive
- encode: faster than manual
- why: it decodes straight into your struct, with no tree in between

## Against protobuf

![tinyklv vs protobuf stacks](https://github.com/arpadav/tinyklv/blob/main/benches/bench_proto.jpg?raw=true)

- not the same bytes: protobuf writes varints; tinyklv writes key, length, value
- faster than each protobuf crate in every row

## Machine

| part | value |
|---|---|
| OS | Ubuntu 24.04.4 LTS |
| kernel | Linux 6.17.0-35-generic |
| CPU | AMD Ryzen 9 9900X |
| board | B850 AI TOP -CF-WCP-ADO |

The data and the chart scripts are in `benches/`.
