# tinyklv: A [Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV) framework in Rust using [`winnow`](https://crates.io/crates/winnow)

***If you are looking for parsing [TLV data (Type-Length-Value)](https://en.wikipedia.org/wiki/Type%E2%80%93length%E2%80%93value), `winnow`, `nom`, as well as other parsing crates already provide this support. KLV is built ontop of TLV.***

`tinyklv` is a Rust implementation of a KLV framework to reduce the amount of boilerplate code required for parsing and encoding KLV data in an agnostic, human-defined manner.

This crate is predominately used for streams of packetized data, like from video feeds or serial ports.
 <!-- Options for handling streams of partial packets is supported. TODO: implement this before adding to README -->

```rust
use tinyklv::Klv;
use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"\x00\x00\x00",
    key(dec = tinyklv::dec::binary::u8,
        enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize,
        enc = tinyklv::enc::binary::u8),
)]
struct Foo {
    #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    name: String,

    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    number: u16,
}

let mut stream1: &[u8] = &[
    // sentinel: 0x00, 0x00, 0x00
    0x00, 0x00, 0x00,
    // packet length: 9 bytes
    0x09,
    // key: 0x01, len: 0x03
    // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    0x01, 0x03,
    // value decoded: "KLV"
    0x4B, 0x4C, 0x56,
    // key: 0x02, len: 0x02
    // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    0x02, 0x02,
    // value decoded: 258
    0x01, 0x02,
];
let stream1_ = stream1.clone();
// decode by seeking sentinel, then decoding data
match Foo::extract(&mut stream1) {
    Ok(foo) => {
        assert_eq!(foo.name, "KLV");
        assert_eq!(foo.number, 258);
    },
    Err(e) => panic!("{}", e),
}
// decode data directly (without seeking sentinel)
match Foo::decode(&mut &stream1_[4..]) {
    Ok(foo) => {
        assert_eq!(foo.name, "KLV");
        assert_eq!(foo.number, 258);
    },
    Err(e) => panic!("{}", e),
}

let mut stream2: &[u8] = &[
    // sentinel: 0x00, 0x00, 0x00
    0x00, 0x00, 0x00,
    // packet length: 18 bytes
    0x12,
    // key: 0x01, len: 0x0C
    // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    0x01, 0x0C,
    // value decoded: "Hello World!"
    0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21,
    // key: 0x02, len: 0x02
    // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    0x02, 0x02,
    // value decoded: 42
    0x00, 0x2A,
];
let stream2_ = stream2.clone();
// decode by seeking sentinel, then decoding data
match Foo::extract(&mut stream2) {
    Ok(foo) => {
        assert_eq!(foo.name, "Hello World!");
        assert_eq!(foo.number, 42);
    },
    Err(e) => panic!("{}", e),
}
// decode data directly (without seeking sentinel)
match Foo::decode(&mut &stream2_[4..]) {
    Ok(foo) => {
        assert_eq!(foo.name, "Hello World!");
        assert_eq!(foo.number, 42);
    },
    Err(e) => panic!("{}", e),
}
```

## Assumptions

* This crate assumes you are familiar with Rust.
* This crate assumes you are familiar with combinator parsers like `winnow` and `nom`. *This crate explicitly uses `winnow` as a backend, so all encoder and decoder functions must be `winnow` compatible*.

## Usage

Please see [tinyklv_common](../tinyklv_common/) for usage examples.

## Why `winnow`? And `winnow` Resources

If not familiar with `winnow`, please refer to the links below.

* [`winnow` Documentation](https://docs.rs/winnow/latest/winnow/)
* [`winnow` Tutorials](https://docs.rs/winnow/latest/winnow/_tutorial/index.html)
* [Why `winnow`](https://docs.rs/winnow/latest/winnow/_topic/why/index.html)

If familiar with `nom` but not `winnow`, please refer to the links below.

* [Migration from `nom` to `winnow`](https://docs.rs/winnow/latest/winnow/_topic/nom/index.html)
* [Rust Parser Benchmarks](https://github.com/rosetta-rs/parse-rosetta-rs/tree/main/examples)

`winnow` uses a slightly different syntax for combinator parsers than `nom`, but it is pretty easy to learn one from the other, since `winnow` is a fork of `nom`. I personally can not speak on the design changes, but after reading some [articles from the `winnow` author (active `nom` contributor)](https://epage.github.io/blog/2023/07/winnow-0-5-the-fastest-rust-parser-combinator-library/) it seems that `winnow` has tried to refactor design decisions from `nom` to optimize for speed and developer experience.

## License

`tinyklv` is licensed under the [MIT License](./LICENSE). [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT).
