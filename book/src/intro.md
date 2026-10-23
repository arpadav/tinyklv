# Introduction

tinyklv is a derive macro for reading and writing [KLV](./what-is-klv.md) (key, length, value) binary data in Rust. It is built on [winnow](https://crates.io/crates/winnow) parser combinators.

- `#[derive(Klv)]` on a struct gives it a decoder and an encoder
- it ships no standard: you choose how each key, length and value is read and written

## Who it is for

Anyone who reads or writes framed binary data in Rust, such as:

- telemetry carried in MPEG-TS video, from airborne and ground sensors
- UAV and satellite command buses
- sensor packets on CAN bus or RS-485
- in-house binary protocols, and their test harnesses

Knowing winnow helps, but is not needed: where a page uses it, the page explains it.

## Dependency

```toml
[dependencies]
tinyklv = "0.2"
```

## Run the examples

Every tutorial page has an example you can run, in the tinyklv repository:

```sh
git clone https://github.com/arpadav/tinyklv
cd tinyklv
cargo run --example book_basics_first_packet   # one page's example
examples/run-examples.sh                        # every example
```

Each page's "Run this example" gives its command and what it prints.

## Stability

tinyklv is pre-1.0, so a minor version may change the API. This book describes 0.2.

**Next:** [How to read this book](./how-to-read.md)
