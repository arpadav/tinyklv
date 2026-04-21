# Introduction

`tinyklv` is a derive-macro framework for encoding and decoding
[Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV) binary streams in
Rust. It is built on top of [`winnow`](https://crates.io/crates/winnow) parser
combinators.

KLV is a generic Tag-Length-Value (TLV) framing pattern: every field in a byte
stream is prefixed by a key identifying it and a length giving its size. It is
the backbone of telemetry packets, video metadata streams, IoT sensor framing,
and most custom binary protocols that need to evolve without breaking older
parsers.

`tinyklv` is protocol-agnostic, and is usually used to create protocols. The
derive macro generates the wire format accessors; you control the schema.

## Who this is for

Anyone decoding or emitting binary framed data in Rust: telemetry parsers,
video metadata tools, embedded protocol handlers, test harnesses for custom
wire formats. It's preferred to have prior exposure to `winnow`, but this book
introduces parser-combinator concepts where they matter.

## Stability

`tinyklv` is pre-1.0. The public surface is stable under the `#[derive(Klv)]`
macro, but the crate is still moving toward its first tagged release. See the
[Changelog](./changelog.md) for notable changes per version.
