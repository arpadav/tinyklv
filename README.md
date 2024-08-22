# tinyklv: A [Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV) framework in Rust using [`winnow`](https://crates.io/crates/winnow)

***If you are looking for parsing [TLV data (Type-Length-Value)](https://en.wikipedia.org/wiki/Type%E2%80%93length%E2%80%93value), `winnow`, `nom`, as well as other parsing crates already provide this support. KLV is built ontop of TLV.***

`tinyklv` is a Rust implementation of a KLV framework to reduce the amount of boilerplate code required for parsing and encoding KLV data in an agnostic, human-defined manner.

This crate is predominately used for streams of packetized data, like from video feeds or serial ports. Options for handling streams of partial packets is supported.

```rust
#[derive(Klv)]
#[klv(
    /// Recognition sentinel
    sentinel = b"\x01",
    /// Required
    key(enc = ::tinyklv::parsers::ber_, dec = someting2),
    /// Required
    len(enc = lsometing, dec = lsometing2),
    /// Optional, default encoder or decoder for specified types
    default(ty = u16, enc = this, dec = that),
    default(ty = f32, enc = foo, dec = bar),
    default(ty = Vec<f64>, enc = me),
)]
struct ExampleStruct {
    #[klv(
        key = b"\x02",
        len = 3,
    )]
    checksum: u16,

    #[klv(
        key = b"\x03",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    val2: String,

    #[klv(
        key = b"\x04",
        len = 3,
        dec = my_str_dec,
    )]
    another_val: String,
}
```

## Assumptions

* This crate assumes you are familiar with Rust.
* This crate assumes you are familiar with combinator parsers like `winnow` and `nom`. *This crate explicitly uses `winnow` as a backend, so all encoder and decoder functions must be `winnow` compatible*.

## Why `winnow`? And `winnow` Resources

If not familiar with `winnow`, please refer to the links below.

* [`winnow` Documentation](https://docs.rs/winnow/latest/winnow/)
* [`winnow` Tutorials](https://docs.rs/winnow/latest/winnow/_tutorial/index.html)
* [Why `winnow`](https://docs.rs/winnow/latest/winnow/_topic/why/index.html)

If familiar with `nom` but not `winnow`, please refer to the links below.

* [Migration from `nom` to `winnow`](https://docs.rs/winnow/latest/winnow/_topic/nom/index.html)
* [Rust Parser Benchmarks](https://github.com/rosetta-rs/parse-rosetta-rs/tree/main/examples)

`winnow` uses a slightly different syntax for combinator parsers than `nom`, but it is pretty easy to learn one from the other, since `winnow` is a fork of `nom`. I personally can not speak on the design changes, but after reading some [articles from the `winnow` author (active `nom` contributor)](https://epage.github.io/blog/2023/07/winnow-0-5-the-fastest-rust-parser-combinator-library/) it seems that `winnow` has tried to refactor design decisions from `nom` to optimize for speed and developer experience.

## Constraints

* `tinyklv` only supports an byte slices as input, as it is meant for parsing of packet streams.
