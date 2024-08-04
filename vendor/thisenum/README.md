# thisenum

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/thisenum.svg)](https://crates.io/crates/thisenum)
<!-- [![Latest Release](https://img.shields.io/github/v/release/arpadav/thisenum)](https://github.com/arpadav/thisenum) -->
<!-- [![Coverage Status](https://coveralls.io/repos/github/arpadav/thisenum/badge.svg?branch=main)](https://coveralls.io/github/arpadav/thisenum?branch=main) -->

The simplest way to assign constant literals to enum arms in Rust! What fun!

Please also see [enum-assoc](https://docs.rs/enum-assoc/latest/enum_assoc/), which is a more robust alternative.

```rust
use thisenum::Const;

#[derive(Const, Debug)]
#[armtype(&[u8])]
/// https://exiftool.org/TagNames/EXIF.html
enum ExifTag {
    // ...
    #[value = b"\x01\x00"]
    ImageWidth,
    #[value = b"\x01\x01"]
    ImageHeight,
    #[value = b"\x01\x02"]
    BitsPerSample,
    #[value = b"\x01\x03"]
    Compression,
    #[value = b"\x01\x06"]
    PhotometricInterpretation,
    // ...
}

assert_eq!(ExifTag::ImageWidth.value(), b"\x01\x00");
assert_eq!(ExifTag::ImageWidth, b"\x01\x00");
```

If each arm is a different type, this is still possible using `ConstEach`:

```rust
use thisenum::ConstEach;

#[derive(ConstEach, Debug)]
enum CustomEnum {
    #[armtype(&[u8])]
    #[value = b"\x01\x00"]
    A,
    // `armtype` is not required, type is inferred
    #[value = "foo"]
    B,
    #[armtype(f32)]
    #[value = 3.14]
    C,
}

assert_eq!(CustomEnum::A.value::<&[u8]>().unwrap(), b"\x01\x00");
assert!(CustomEnum::B.value::<&str>().is_some());
assert_eq!(CustomEnum::B.value::<&str>().unwrap(), &"foo");
assert_eq!(CustomEnum::B.value::<&str>(), Some("foo").as_ref());
assert_eq!(CustomEnum::C.value::<f32>().unwrap(), &3.14);
// or on failure
assert!(CustomEnum::C.value::<i32>().is_none());
```

## License

`thisenum` is released under the [MIT License](LICENSE) [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT).