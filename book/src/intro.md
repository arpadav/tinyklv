# Introduction

`tinyklv` is a derive-macro framework for encoding and decoding
[Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV) binary streams in
Rust. It is built on top of [`winnow`](https://crates.io/crates/winnow) parser
combinators.

`tinyklv` is protocol-agnostic, and is usually used to create protocols. The
derive macro generates encoder/decoder trait implementations, allowing for
byte-stream data to be generated and parsed on the fly.

## Who this is for

Anyone decoding or emitting binary framed data in Rust. Some examples may
include telemetry parsers, video metadata tools, embedded protocol handlers,
and test harnesses for custom binary formats. 

Prior exposure to `winnow` helps but is not required. 
This book introduces parser-combinator concepts where they matter.

## Quick start

```sh
cargo add tinyklv
```

Then head to [Tutorial 01 - First packet](./tutorial/fundamentals/01-first-packet.md).

## Stability

`tinyklv` is pre-1.0. The public surface is stable under the `#[derive(Klv)]`
macro, but the crate is still moving toward its first tagged release.
