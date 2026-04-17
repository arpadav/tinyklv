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

`tinyklv` is protocol-agnostic. It ships no baked-in standards - you declare
your keys, your length encoding, your sentinel, and the codec for each field
via attributes on a struct. The derive macro generates the wire format
accessors; you control the schema.

## What this book covers

- **Getting Started** - install, write your first packet, run a roundtrip.
- **Tutorials** - 15 progressively deeper examples, Beginner → Advanced.
- **Reference** - every attribute, every trait, every built-in codec.
- **Architecture** - how the proc-macro expands; why `enc_owned!` exists.

## Who this is for

Anyone decoding or emitting binary framed data in Rust: telemetry parsers,
video metadata tools, embedded protocol handlers, test harnesses for custom
wire formats. You do not need prior exposure to `winnow` - the book introduces
parser-combinator concepts where they matter, and hides them where they don't.

## Stability

`tinyklv` is pre-1.0. The public surface is stable under the `#[derive(Klv)]`
macro, but the crate is still moving toward its first tagged release. See the
[Changelog](./changelog.md) for notable changes per version.
