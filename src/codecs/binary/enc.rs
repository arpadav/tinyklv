//! Binary encode codecs for KLV data
//!
//! Provides encoders for converting Rust primitive types into raw byte slices
//! suitable for KLV value fields. All encoders return `Vec<u8>` and are
//! compatible with the `#[klv(enc = ...)]` derive macro attribute
//!
//! Includes:
//! * Native-endian, big-endian, and little-endian encoders for all standard
//!   integer and float types
//! * `_from_usize` variants for encoding length fields as typed integers
//! * Variable-length ("lengthed") encoders that truncate or zero-pad output
//!   to an exact byte count
//!
//! Author: aav
macro_rules! encode_native {
    ($ty:ty) => {
        encode_native!($ty, "", "");
    };

    ($ty:ty, $doc_native:literal, $doc_from_usize:literal) => {
        paste::paste! {
            #[doc = $doc_native]
            #[inline(always)]
            pub fn [<$ty>](input: $ty) -> Vec<u8> {
                #[cfg(target_endian = "big")]
                { input.to_be_bytes().to_vec() }
                #[cfg(target_endian = "little")]
                { input.to_le_bytes().to_vec() }
            }

            #[doc = $doc_from_usize]
            #[inline(always)]
            pub fn [<$ty _from_usize>](input: usize) -> Vec<u8> {
                #[cfg(target_endian = "big")]
                { (input as $ty).to_be_bytes().to_vec() }
                #[cfg(target_endian = "little")]
                { (input as $ty).to_le_bytes().to_vec() }
            }
        }
    };
}

macro_rules! encode_endian {
    ($ty:ty) => {
        encode_endian!($ty, "", "", "", "");
    };

    ($ty:ty, $doc_be:literal, $doc_le:literal, $doc_be_usize:literal, $doc_le_usize:literal) => {
        paste::paste! {
            #[doc = $doc_be]
            #[inline(always)]
            pub fn [<be_ $ty>](input: $ty) -> Vec<u8> {
                input.to_be_bytes().to_vec()
            }

            #[doc = $doc_be_usize]
            #[inline(always)]
            pub fn [<be_ $ty _from_usize>](input: usize) -> Vec<u8> {
                (input as $ty).to_be_bytes().to_vec()
            }

            #[doc = $doc_le]
            #[inline(always)]
            pub fn [<le_ $ty>](input: $ty) -> Vec<u8> {
                input.to_le_bytes().to_vec()
            }

            #[doc = $doc_le_usize]
            #[inline(always)]
            pub fn [<le_ $ty _from_usize>](input: usize) -> Vec<u8> {
                (input as $ty).to_le_bytes().to_vec()
            }
        }
    };
}

macro_rules! encode_lengthed {
    ($ty:ty) => {
        encode_lengthed!($ty, "", "");
    };

    ($ty:ty, $doc_be_lengthed:literal, $doc_le_lengthed:literal) => {
        paste::paste! {
            #[doc = $doc_be_lengthed]
            #[inline(always)]
            pub fn [<be_ $ty _lengthed>](len: usize) -> impl Fn($ty) -> Vec<u8> {
                move |input: $ty| {
                    let bytes = input.to_be_bytes();
                    let start = bytes.len().saturating_sub(len);
                    let mut result = Vec::with_capacity(len);
                    result.extend_from_slice(&bytes[start..]);
                    result.splice(0..0, std::iter::repeat(0).take(len - result.len()));
                    result
                }
            }

            #[doc = $doc_le_lengthed]
            #[inline(always)]
            pub fn [<le_ $ty _lengthed>](len: usize) -> impl Fn($ty) -> Vec<u8> {
                move |input: $ty| {
                    let mut v = input.to_le_bytes().to_vec();
                    v.resize(len, 0);
                    v
                }
            }
        }
    };
}

encode_native!(
    u8,
    "Encodes a [`prim@u8`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u8(42_u8);
assert_eq!(encoded.len(), std::mem::size_of::<u8>());
```",
    "Encodes a `usize` value as a [`prim@u8`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u8_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u8>());
```"
);

encode_lengthed!(
    u8,
    "Encodes a [`prim@u8`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_u8_lengthed(1);
assert_eq!(encode(0x01_u8), vec![0x01]);
let padded = tinyklv::codecs::binary::enc::be_u8_lengthed(3);
assert_eq!(padded(0x01_u8), vec![0x00, 0x00, 0x01]);
```",
    "Encodes a [`prim@u8`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_u8_lengthed(1);
assert_eq!(encode(0x01_u8), vec![0x01]);
let padded = tinyklv::codecs::binary::enc::le_u8_lengthed(3);
assert_eq!(padded(0x01_u8), vec![0x01, 0x00, 0x00]);
```"
);

encode_native!(
    u16,
    "Encodes a [`prim@u16`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u16(42_u16);
assert_eq!(encoded.len(), std::mem::size_of::<u16>());
```",
    "Encodes a `usize` value as a [`prim@u16`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u16_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u16>());
```"
);

encode_endian!(
    u16,
    "Encodes a [`prim@u16`] value using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_u16(0x0102_u16), vec![0x01, 0x02]);
```",
    "Encodes a [`prim@u16`] value using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_u16(0x0102_u16), vec![0x02, 0x01]);
```",
    "Encodes a `usize` value as a [`prim@u16`] using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_u16_from_usize(0x0102_usize), vec![0x01, 0x02]);
```",
    "Encodes a `usize` value as a [`prim@u16`] using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_u16_from_usize(0x0102_usize), vec![0x02, 0x01]);
```"
);

encode_lengthed!(
    u16,
    "Encodes a [`prim@u16`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_u16_lengthed(2);
assert_eq!(encode(0x0102_u16), vec![0x01, 0x02]);
let truncated = tinyklv::codecs::binary::enc::be_u16_lengthed(1);
assert_eq!(truncated(0x0102_u16), vec![0x02]);
```",
    "Encodes a [`prim@u16`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_u16_lengthed(2);
assert_eq!(encode(0x0102_u16), vec![0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_u16_lengthed(3);
assert_eq!(padded(0x0102_u16), vec![0x02, 0x01, 0x00]);
```"
);

encode_native!(
    u32,
    "Encodes a [`prim@u32`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u32(42_u32);
assert_eq!(encoded.len(), std::mem::size_of::<u32>());
```",
    "Encodes a `usize` value as a [`prim@u32`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u32_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u32>());
```"
);

encode_endian!(
    u32,
    "Encodes a [`prim@u32`] value using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_u32(0x01020304_u32), vec![0x01, 0x02, 0x03, 0x04]);
```",
    "Encodes a [`prim@u32`] value using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_u32(0x01020304_u32), vec![0x04, 0x03, 0x02, 0x01]);
```",
    "Encodes a `usize` value as a [`prim@u32`] using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_u32_from_usize(0x0102_usize), vec![0x00, 0x00, 0x01, 0x02]);
```",
    "Encodes a `usize` value as a [`prim@u32`] using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_u32_from_usize(0x0102_usize), vec![0x02, 0x01, 0x00, 0x00]);
```"
);

encode_lengthed!(
    u32,
    "Encodes a [`prim@u32`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_u32_lengthed(4);
assert_eq!(encode(0x01020304_u32), vec![0x01, 0x02, 0x03, 0x04]);
let truncated = tinyklv::codecs::binary::enc::be_u32_lengthed(2);
assert_eq!(truncated(0x01020304_u32), vec![0x03, 0x04]);
```",
    "Encodes a [`prim@u32`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_u32_lengthed(4);
assert_eq!(encode(0x01020304_u32), vec![0x04, 0x03, 0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_u32_lengthed(5);
assert_eq!(padded(0x01020304_u32), vec![0x04, 0x03, 0x02, 0x01, 0x00]);
```"
);

encode_native!(
    u64,
    "Encodes a [`prim@u64`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u64(42_u64);
assert_eq!(encoded.len(), std::mem::size_of::<u64>());
```",
    "Encodes a `usize` value as a [`prim@u64`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u64_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u64>());
```"
);

encode_endian!(
    u64,
    "Encodes a [`prim@u64`] value using big-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::be_u64(0x0102030405060708_u64),
    vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
);
```",
    "Encodes a [`prim@u64`] value using little-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::le_u64(0x0102030405060708_u64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
);
```",
    "Encodes a `usize` value as a [`prim@u64`] using big-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::be_u64_from_usize(0x0102_usize),
    vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02],
);
```",
    "Encodes a `usize` value as a [`prim@u64`] using little-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::le_u64_from_usize(0x0102_usize),
    vec![0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
);
```"
);

encode_lengthed!(
    u64,
    "Encodes a [`prim@u64`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_u64_lengthed(8);
assert_eq!(
    encode(0x0102030405060708_u64),
    vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
);
let truncated = tinyklv::codecs::binary::enc::be_u64_lengthed(2);
assert_eq!(truncated(0x0102030405060708_u64), vec![0x07, 0x08]);
```",
    "Encodes a [`prim@u64`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_u64_lengthed(8);
assert_eq!(
    encode(0x0102030405060708_u64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
);
let padded = tinyklv::codecs::binary::enc::le_u64_lengthed(10);
assert_eq!(
    padded(0x0102030405060708_u64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00],
);
```"
);

encode_native!(
    u128,
    "Encodes a [`prim@u128`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u128(42_u128);
assert_eq!(encoded.len(), std::mem::size_of::<u128>());
```",
    "Encodes a `usize` value as a [`prim@u128`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::u128_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u128>());
```"
);

encode_endian!(
    u128,
    "Encodes a [`prim@u128`] value using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_u128(1_u128);
assert_eq!(encoded[15], 0x01);
assert_eq!(encoded[0], 0x00);
```",
    "Encodes a [`prim@u128`] value using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_u128(1_u128);
assert_eq!(encoded[0], 0x01);
assert_eq!(encoded[15], 0x00);
```",
    "Encodes a `usize` value as a [`prim@u128`] using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_u128_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u128>());
assert_eq!(encoded[15], 0x01);
```",
    "Encodes a `usize` value as a [`prim@u128`] using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_u128_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<u128>());
assert_eq!(encoded[0], 0x01);
```"
);

encode_lengthed!(
    u128,
    "Encodes a [`prim@u128`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_u128_lengthed(2);
assert_eq!(encode(0x0102_u128), vec![0x01, 0x02]);
let truncated = tinyklv::codecs::binary::enc::be_u128_lengthed(1);
assert_eq!(truncated(0x0102_u128), vec![0x02]);
```",
    "Encodes a [`prim@u128`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_u128_lengthed(2);
assert_eq!(encode(0x0102_u128), vec![0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_u128_lengthed(3);
assert_eq!(padded(0x0102_u128), vec![0x02, 0x01, 0x00]);
```"
);

encode_native!(
    i8,
    "Encodes an [`prim@i8`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i8(42_i8);
assert_eq!(encoded.len(), std::mem::size_of::<i8>());
```",
    "Encodes a `usize` value as an [`prim@i8`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i8_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i8>());
```"
);

encode_lengthed!(
    i8,
    "Encodes an [`prim@i8`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_i8_lengthed(1);
assert_eq!(encode(0x01_i8), vec![0x01]);
let padded = tinyklv::codecs::binary::enc::be_i8_lengthed(3);
assert_eq!(padded(0x01_i8), vec![0x00, 0x00, 0x01]);
```",
    "Encodes an [`prim@i8`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_i8_lengthed(1);
assert_eq!(encode(0x01_i8), vec![0x01]);
let padded = tinyklv::codecs::binary::enc::le_i8_lengthed(3);
assert_eq!(padded(0x01_i8), vec![0x01, 0x00, 0x00]);
```"
);

encode_native!(
    i16,
    "Encodes an [`prim@i16`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i16(42_i16);
assert_eq!(encoded.len(), std::mem::size_of::<i16>());
```",
    "Encodes a `usize` value as an [`prim@i16`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i16_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i16>());
```"
);

encode_endian!(
    i16,
    "Encodes an [`prim@i16`] value using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_i16(0x0102_i16), vec![0x01, 0x02]);
```",
    "Encodes an [`prim@i16`] value using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_i16(0x0102_i16), vec![0x02, 0x01]);
```",
    "Encodes a `usize` value as an [`prim@i16`] using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_i16_from_usize(0x0102_usize), vec![0x01, 0x02]);
```",
    "Encodes a `usize` value as an [`prim@i16`] using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_i16_from_usize(0x0102_usize), vec![0x02, 0x01]);
```"
);

encode_lengthed!(
    i16,
    "Encodes an [`prim@i16`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_i16_lengthed(2);
assert_eq!(encode(0x0102_i16), vec![0x01, 0x02]);
let truncated = tinyklv::codecs::binary::enc::be_i16_lengthed(1);
assert_eq!(truncated(0x0102_i16), vec![0x02]);
```",
    "Encodes an [`prim@i16`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_i16_lengthed(2);
assert_eq!(encode(0x0102_i16), vec![0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_i16_lengthed(3);
assert_eq!(padded(0x0102_i16), vec![0x02, 0x01, 0x00]);
```"
);

encode_native!(
    i32,
    "Encodes an [`prim@i32`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i32(42_i32);
assert_eq!(encoded.len(), std::mem::size_of::<i32>());
```",
    "Encodes a `usize` value as an [`prim@i32`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i32_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i32>());
```"
);

encode_endian!(
    i32,
    "Encodes an [`prim@i32`] value using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_i32(0x01020304_i32), vec![0x01, 0x02, 0x03, 0x04]);
```",
    "Encodes an [`prim@i32`] value using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_i32(0x01020304_i32), vec![0x04, 0x03, 0x02, 0x01]);
```",
    "Encodes a `usize` value as an [`prim@i32`] using big-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::be_i32_from_usize(0x0102_usize), vec![0x00, 0x00, 0x01, 0x02]);
```",
    "Encodes a `usize` value as an [`prim@i32`] using little-endian byte order.

# Example

```
assert_eq!(tinyklv::codecs::binary::enc::le_i32_from_usize(0x0102_usize), vec![0x02, 0x01, 0x00, 0x00]);
```"
);

encode_lengthed!(
    i32,
    "Encodes an [`prim@i32`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_i32_lengthed(4);
assert_eq!(encode(0x01020304_i32), vec![0x01, 0x02, 0x03, 0x04]);
let truncated = tinyklv::codecs::binary::enc::be_i32_lengthed(2);
assert_eq!(truncated(0x01020304_i32), vec![0x03, 0x04]);
```",
    "Encodes an [`prim@i32`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_i32_lengthed(4);
assert_eq!(encode(0x01020304_i32), vec![0x04, 0x03, 0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_i32_lengthed(5);
assert_eq!(padded(0x01020304_i32), vec![0x04, 0x03, 0x02, 0x01, 0x00]);
```"
);

encode_native!(
    i64,
    "Encodes an [`prim@i64`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i64(42_i64);
assert_eq!(encoded.len(), std::mem::size_of::<i64>());
```",
    "Encodes a `usize` value as an [`prim@i64`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i64_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i64>());
```"
);

encode_endian!(
    i64,
    "Encodes an [`prim@i64`] value using big-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::be_i64(0x0102030405060708_i64),
    vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
);
```",
    "Encodes an [`prim@i64`] value using little-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::le_i64(0x0102030405060708_i64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
);
```",
    "Encodes a `usize` value as an [`prim@i64`] using big-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::be_i64_from_usize(0x0102_usize),
    vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02],
);
```",
    "Encodes a `usize` value as an [`prim@i64`] using little-endian byte order.

# Example

```
assert_eq!(
    tinyklv::codecs::binary::enc::le_i64_from_usize(0x0102_usize),
    vec![0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
);
```"
);

encode_lengthed!(
    i64,
    "Encodes an [`prim@i64`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_i64_lengthed(8);
assert_eq!(
    encode(0x0102030405060708_i64),
    vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
);
let truncated = tinyklv::codecs::binary::enc::be_i64_lengthed(2);
assert_eq!(truncated(0x0102030405060708_i64), vec![0x07, 0x08]);
```",
    "Encodes an [`prim@i64`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_i64_lengthed(8);
assert_eq!(
    encode(0x0102030405060708_i64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
);
let padded = tinyklv::codecs::binary::enc::le_i64_lengthed(10);
assert_eq!(
    padded(0x0102030405060708_i64),
    vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00],
);
```"
);

encode_native!(
    i128,
    "Encodes an [`prim@i128`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i128(42_i128);
assert_eq!(encoded.len(), std::mem::size_of::<i128>());
```",
    "Encodes a `usize` value as an [`prim@i128`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::i128_from_usize(42_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i128>());
```"
);

encode_endian!(
    i128,
    "Encodes an [`prim@i128`] value using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_i128(1_i128);
assert_eq!(encoded[15], 0x01);
assert_eq!(encoded[0], 0x00);
```",
    "Encodes an [`prim@i128`] value using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_i128(1_i128);
assert_eq!(encoded[0], 0x01);
assert_eq!(encoded[15], 0x00);
```",
    "Encodes a `usize` value as an [`prim@i128`] using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_i128_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i128>());
assert_eq!(encoded[15], 0x01);
```",
    "Encodes a `usize` value as an [`prim@i128`] using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_i128_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<i128>());
assert_eq!(encoded[0], 0x01);
```"
);

encode_lengthed!(
    i128,
    "Encodes an [`prim@i128`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::be_i128_lengthed(2);
assert_eq!(encode(0x0102_i128), vec![0x01, 0x02]);
let truncated = tinyklv::codecs::binary::enc::be_i128_lengthed(1);
assert_eq!(truncated(0x0102_i128), vec![0x02]);
```",
    "Encodes an [`prim@i128`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
let encode = tinyklv::codecs::binary::enc::le_i128_lengthed(2);
assert_eq!(encode(0x0102_i128), vec![0x02, 0x01]);
let padded = tinyklv::codecs::binary::enc::le_i128_lengthed(3);
assert_eq!(padded(0x0102_i128), vec![0x02, 0x01, 0x00]);
```"
);

encode_native!(
    f32,
    "Encodes an [`prim@f32`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::f32(1.0_f32);
assert_eq!(encoded.len(), std::mem::size_of::<f32>());
```",
    "Encodes a `usize` value as an [`prim@f32`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::f32_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f32>());
```"
);

encode_endian!(
    f32,
    "Encodes an [`prim@f32`] value using big-endian byte order.

# Example

```
// 1.0_f32 in IEEE 754 big-endian: 0x3F800000
assert_eq!(tinyklv::codecs::binary::enc::be_f32(1.0_f32), vec![0x3F, 0x80, 0x00, 0x00]);
```",
    "Encodes an [`prim@f32`] value using little-endian byte order.

# Example

```
// 1.0_f32 in IEEE 754 little-endian: 0x0000803F
assert_eq!(tinyklv::codecs::binary::enc::le_f32(1.0_f32), vec![0x00, 0x00, 0x80, 0x3F]);
```",
    "Encodes a `usize` value as an [`prim@f32`] using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_f32_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f32>());
```",
    "Encodes a `usize` value as an [`prim@f32`] using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_f32_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f32>());
```"
);

encode_lengthed!(
    f32,
    "Encodes an [`prim@f32`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
// 1.0_f32 in IEEE 754 big-endian: 0x3F800000
let encode = tinyklv::codecs::binary::enc::be_f32_lengthed(4);
assert_eq!(encode(1.0_f32), vec![0x3F, 0x80, 0x00, 0x00]);
let truncated = tinyklv::codecs::binary::enc::be_f32_lengthed(2);
assert_eq!(truncated(1.0_f32), vec![0x00, 0x00]);
```",
    "Encodes an [`prim@f32`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
// 1.0_f32 in IEEE 754 little-endian: 0x0000803F
let encode = tinyklv::codecs::binary::enc::le_f32_lengthed(4);
assert_eq!(encode(1.0_f32), vec![0x00, 0x00, 0x80, 0x3F]);
let padded = tinyklv::codecs::binary::enc::le_f32_lengthed(5);
assert_eq!(padded(1.0_f32), vec![0x00, 0x00, 0x80, 0x3F, 0x00]);
```"
);

encode_native!(
    f64,
    "Encodes an [`prim@f64`] value using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::f64(1.0_f64);
assert_eq!(encoded.len(), std::mem::size_of::<f64>());
```",
    "Encodes a `usize` value as an [`prim@f64`] using native-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::f64_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f64>());
```"
);

encode_endian!(
    f64,
    "Encodes an [`prim@f64`] value using big-endian byte order.

# Example

```
// 1.0_f64 in IEEE 754 big-endian: 0x3FF0000000000000
assert_eq!(
    tinyklv::codecs::binary::enc::be_f64(1.0_f64),
    vec![0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
);
```",
    "Encodes an [`prim@f64`] value using little-endian byte order.

# Example

```
// 1.0_f64 in IEEE 754 little-endian: 0x000000000000F03F
assert_eq!(
    tinyklv::codecs::binary::enc::le_f64(1.0_f64),
    vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F],
);
```",
    "Encodes a `usize` value as an [`prim@f64`] using big-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::be_f64_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f64>());
```",
    "Encodes a `usize` value as an [`prim@f64`] using little-endian byte order.

# Example

```
let encoded = tinyklv::codecs::binary::enc::le_f64_from_usize(1_usize);
assert_eq!(encoded.len(), std::mem::size_of::<f64>());
```"
);

encode_lengthed!(
    f64,
    "Encodes an [`prim@f64`] as big-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
// 1.0_f64 in IEEE 754 big-endian: 0x3FF0000000000000
let encode = tinyklv::codecs::binary::enc::be_f64_lengthed(8);
assert_eq!(
    encode(1.0_f64),
    vec![0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
);
let truncated = tinyklv::codecs::binary::enc::be_f64_lengthed(2);
assert_eq!(truncated(1.0_f64), vec![0x00, 0x00]);
```",
    "Encodes an [`prim@f64`] as little-endian bytes, truncated or zero-padded to `len` bytes.

# Example

```
// 1.0_f64 in IEEE 754 little-endian: 0x000000000000F03F
let encode = tinyklv::codecs::binary::enc::le_f64_lengthed(8);
assert_eq!(
    encode(1.0_f64),
    vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F],
);
let padded = tinyklv::codecs::binary::enc::le_f64_lengthed(10);
assert_eq!(
    padded(1.0_f64),
    vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00],
);
```"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_some() {
        let output1 = vec![0x00, 0x01, 0xE0, 0xFF, 0xFF];
        let output2 = vec![0x00, 0x01, 0xE0, 0xFF];
        let output3 = vec![0x01, 0xE0, 0xFF];
        assert_eq!(be_u32_lengthed(5)(0x01E0FFFF), output1);
        assert_eq!(be_u32_lengthed(4)(0x0001E0FF), output2);
        assert_eq!(be_u32_lengthed(3)(0x0001E0FF), output3);

        let output1 = [0xE0, 0x01, 0xFF, 0xFF, 0xFF];
        let output2 = [0x01];
        assert_eq!(le_u16_lengthed(5)(480)[..2], output1[..2]);
        assert_eq!(le_u16_lengthed(1)(1)[..1], output2[..1]);

        let output1 = [0xE0, 0x01, 0xFF, 0xFF, 0xFF];
        let output2 = [0x01];
        let output3 = [0x01, 0x02, 0x03];
        assert_eq!(le_u32_lengthed(5)(4_294_902_240)[..4], output1[..4]);
        assert_eq!(le_u32_lengthed(1)(1)[..1], output2[..1]);
        assert_eq!(le_u32_lengthed(3)(197_121)[..3], output3[..3]);

        let output1 = [
            0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let output2 = [0x01];
        let output3 = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(le_u64_lengthed(16)(1_099_511_562_720)[..16], output1[..16]);
        assert_eq!(le_u64_lengthed(1)(1)[..1], output2[..1]);
        assert_eq!(le_u64_lengthed(7)(1_976_943_448_883_713)[..7], output3[..7]);

        let output1 = [0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
        let output2 = [0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
        let output3 = [0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
        assert_eq!(
            be_u64_lengthed(9)(0x00_01_E0_FF_FF_00_00_00)[..9],
            output1[..9]
        );
        assert_eq!(
            be_u64_lengthed(8)(0x00_00_01_E0_FF_FF_00_00)[..8],
            output2[..8]
        );
        assert_eq!(
            be_u64_lengthed(7)(0x00_00_00_01_E0_FF_FF_00)[..7],
            output3[..7]
        );
    }

    use crate::codecs::binary::dec;
    use rand;
    use rand_distr::Distribution;

    #[test]
    fn randoms() {
        const TRIALS: usize = 10;

        const NBYTES_8: usize = 1;
        const NBYTES_16: usize = 2;
        const NBYTES_32: usize = 4;
        const NBYTES_64: usize = 8;
        const NBYTES_128: usize = 16;

        let random_usize: Box<dyn Fn(usize) -> Vec<usize>> = Box::new(|max| {
            let pareto = rand_distr::Pareto::new(2.0, 1.0).unwrap();
            let mut rng = rand::thread_rng();
            (0..TRIALS)
                .map(|_| (pareto.sample(&mut rng) as usize).clamp(1, max))
                .collect()
        });
        let random_f32: Box<dyn Fn() -> Vec<f32>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<f32>()).collect());
        let random_f64: Box<dyn Fn() -> Vec<f64>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<f64>()).collect());
        let random_u8: Box<dyn Fn() -> Vec<u8>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<u8>()).collect());
        let random_u16: Box<dyn Fn() -> Vec<u16>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<u16>()).collect());
        let random_u32: Box<dyn Fn() -> Vec<u32>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<u32>()).collect());
        let random_u64: Box<dyn Fn() -> Vec<u64>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<u64>()).collect());
        let random_u128: Box<dyn Fn() -> Vec<u128>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<u128>()).collect());
        let random_i8: Box<dyn Fn() -> Vec<i8>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<i8>()).collect());
        let random_i16: Box<dyn Fn() -> Vec<i16>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<i16>()).collect());
        let random_i32: Box<dyn Fn() -> Vec<i32>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<i32>()).collect());
        let random_i64: Box<dyn Fn() -> Vec<i64>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<i64>()).collect());
        let random_i128: Box<dyn Fn() -> Vec<i128>> =
            Box::new(|| (0..TRIALS).map(|_| rand::random::<i128>()).collect());

        random_f32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::f32(&mut f32(*x).as_slice()).unwrap()));
        random_f32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_f32(&mut be_f32(*x).as_slice()).unwrap()));
        random_f32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_f32(&mut le_f32(*x).as_slice()).unwrap()));
        random_f32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_f32_lengthed(NBYTES_32)(&mut be_f32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_f32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_f32_lengthed(NBYTES_32)(&mut le_f32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_f32().iter().for_each(|x| {
            random_usize(NBYTES_32 * NBYTES_32).iter().for_each(|y| {
                match *y < NBYTES_32 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_f32_lengthed(*y)(*x),
                            be_f32_lengthed(*y)(
                                dec::be_f32_lengthed(*y)(&mut be_f32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_f32_lengthed(*y)(*x),
                            le_f32_lengthed(*y)(
                                dec::le_f32_lengthed(*y)(&mut le_f32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_f32_lengthed(*y)(&mut be_f32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_f32_lengthed(*y)(&mut le_f32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_f64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::f64(&mut f64(*x).as_slice()).unwrap()));
        random_f64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_f64(&mut be_f64(*x).as_slice()).unwrap()));
        random_f64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_f64(&mut le_f64(*x).as_slice()).unwrap()));
        random_f64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_f64_lengthed(NBYTES_64)(&mut be_f64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_f64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_f64_lengthed(NBYTES_64)(&mut le_f64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_f64().iter().for_each(|x| {
            random_usize(NBYTES_64 * NBYTES_64).iter().for_each(|y| {
                match *y < NBYTES_64 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_f64_lengthed(*y)(*x),
                            be_f64_lengthed(*y)(
                                dec::be_f64_lengthed(*y)(&mut be_f64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_f64_lengthed(*y)(*x),
                            le_f64_lengthed(*y)(
                                dec::le_f64_lengthed(*y)(&mut le_f64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_f64_lengthed(*y)(&mut be_f64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_f64_lengthed(*y)(&mut le_f64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_u8()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::u8(&mut u8(*x).as_slice()).unwrap()));
        random_u8().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_u8_lengthed(NBYTES_8)(&mut be_u8_lengthed(NBYTES_8)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u8().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_u8_lengthed(NBYTES_8)(&mut le_u8_lengthed(NBYTES_8)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u8().iter().for_each(|x| {
            random_usize(4).iter().for_each(|y| {
                assert_eq!(
                    x,
                    &dec::be_u8_lengthed(*y)(&mut be_u8_lengthed(*y)(*x).as_slice()).unwrap()
                );
                assert_eq!(
                    x,
                    &dec::le_u8_lengthed(*y)(&mut le_u8_lengthed(*y)(*x).as_slice()).unwrap()
                );
            })
        });

        random_u16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::u16(&mut u16(*x).as_slice()).unwrap()));
        random_u16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_u16(&mut be_u16(*x).as_slice()).unwrap()));
        random_u16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_u16(&mut le_u16(*x).as_slice()).unwrap()));
        random_u16().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_u16_lengthed(NBYTES_16)(&mut be_u16_lengthed(NBYTES_16)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u16().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_u16_lengthed(NBYTES_16)(&mut le_u16_lengthed(NBYTES_16)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u16().iter().for_each(|x| {
            random_usize(NBYTES_16 * NBYTES_16).iter().for_each(|y| {
                match *y < NBYTES_16 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_u16_lengthed(*y)(*x),
                            be_u16_lengthed(*y)(
                                dec::be_u16_lengthed(*y)(&mut be_u16_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_u16_lengthed(*y)(*x),
                            le_u16_lengthed(*y)(
                                dec::le_u16_lengthed(*y)(&mut le_u16_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_u16_lengthed(*y)(&mut be_u16_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_u16_lengthed(*y)(&mut le_u16_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_u32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::u32(&mut u32(*x).as_slice()).unwrap()));
        random_u32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_u32(&mut be_u32(*x).as_slice()).unwrap()));
        random_u32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_u32(&mut le_u32(*x).as_slice()).unwrap()));
        random_u32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_u32_lengthed(NBYTES_32)(&mut be_u32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_u32_lengthed(NBYTES_32)(&mut le_u32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u32().iter().for_each(|x| {
            random_usize(NBYTES_32 * NBYTES_32).iter().for_each(|y| {
                match *y < NBYTES_32 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_u32_lengthed(*y)(*x),
                            be_u32_lengthed(*y)(
                                dec::be_u32_lengthed(*y)(&mut be_u32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_u32_lengthed(*y)(*x),
                            le_u32_lengthed(*y)(
                                dec::le_u32_lengthed(*y)(&mut le_u32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_u32_lengthed(*y)(&mut be_u32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_u32_lengthed(*y)(&mut le_u32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_u64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::u64(&mut u64(*x).as_slice()).unwrap()));
        random_u64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_u64(&mut be_u64(*x).as_slice()).unwrap()));
        random_u64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_u64(&mut le_u64(*x).as_slice()).unwrap()));
        random_u64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_u64_lengthed(NBYTES_64)(&mut be_u64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_u64_lengthed(NBYTES_64)(&mut le_u64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_u64().iter().for_each(|x| {
            random_usize(NBYTES_64 * NBYTES_64).iter().for_each(|y| {
                match *y < NBYTES_64 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_u64_lengthed(*y)(*x),
                            be_u64_lengthed(*y)(
                                dec::be_u64_lengthed(*y)(&mut be_u64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_u64_lengthed(*y)(*x),
                            le_u64_lengthed(*y)(
                                dec::le_u64_lengthed(*y)(&mut le_u64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_u64_lengthed(*y)(&mut be_u64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_u64_lengthed(*y)(&mut le_u64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_u128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::u128(&mut u128(*x).as_slice()).unwrap()));
        random_u128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_u128(&mut be_u128(*x).as_slice()).unwrap()));
        random_u128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_u128(&mut le_u128(*x).as_slice()).unwrap()));
        random_u128().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_u128_lengthed(NBYTES_128)(
                    &mut be_u128_lengthed(NBYTES_128)(*x).as_slice()
                )
                .unwrap()
            )
        });
        random_u128().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_u128_lengthed(NBYTES_128)(
                    &mut le_u128_lengthed(NBYTES_128)(*x).as_slice()
                )
                .unwrap()
            )
        });
        random_u128().iter().for_each(|x| {
            random_usize(NBYTES_128 * NBYTES_128).iter().for_each(|y| {
                match *y < NBYTES_128 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_u128_lengthed(*y)(*x),
                            be_u128_lengthed(*y)(
                                dec::be_u128_lengthed(*y)(&mut be_u128_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_u128_lengthed(*y)(*x),
                            le_u128_lengthed(*y)(
                                dec::le_u128_lengthed(*y)(&mut le_u128_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_u128_lengthed(*y)(&mut be_u128_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_u128_lengthed(*y)(&mut le_u128_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_i8()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::i8(&mut i8(*x).as_slice()).unwrap()));
        random_i8().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_i8_lengthed(NBYTES_8)(&mut be_i8_lengthed(NBYTES_8)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i8().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_i8_lengthed(NBYTES_8)(&mut le_i8_lengthed(NBYTES_8)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i8().iter().for_each(|x| {
            random_usize(4).iter().for_each(|y| {
                assert_eq!(
                    x,
                    &dec::be_i8_lengthed(*y)(&mut be_i8_lengthed(*y)(*x).as_slice()).unwrap()
                );
                assert_eq!(
                    x,
                    &dec::le_i8_lengthed(*y)(&mut le_i8_lengthed(*y)(*x).as_slice()).unwrap()
                );
            })
        });

        random_i16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::i16(&mut i16(*x).as_slice()).unwrap()));
        random_i16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_i16(&mut be_i16(*x).as_slice()).unwrap()));
        random_i16()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_i16(&mut le_i16(*x).as_slice()).unwrap()));
        random_i16().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_i16_lengthed(NBYTES_16)(&mut be_i16_lengthed(NBYTES_16)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i16().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_i16_lengthed(NBYTES_16)(&mut le_i16_lengthed(NBYTES_16)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i16().iter().for_each(|x| {
            random_usize(NBYTES_16 * NBYTES_16).iter().for_each(|y| {
                match *y < NBYTES_16 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_i16_lengthed(*y)(*x),
                            be_i16_lengthed(*y)(
                                dec::be_i16_lengthed(*y)(&mut be_i16_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_i16_lengthed(*y)(*x),
                            le_i16_lengthed(*y)(
                                dec::le_i16_lengthed(*y)(&mut le_i16_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_i16_lengthed(*y)(&mut be_i16_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_i16_lengthed(*y)(&mut le_i16_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_i32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::i32(&mut i32(*x).as_slice()).unwrap()));
        random_i32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_i32(&mut be_i32(*x).as_slice()).unwrap()));
        random_i32()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_i32(&mut le_i32(*x).as_slice()).unwrap()));
        random_i32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_i32_lengthed(NBYTES_32)(&mut be_i32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i32().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_i32_lengthed(NBYTES_32)(&mut le_i32_lengthed(NBYTES_32)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i32().iter().for_each(|x| {
            random_usize(NBYTES_32 * NBYTES_32).iter().for_each(|y| {
                match *y < NBYTES_32 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_i32_lengthed(*y)(*x),
                            be_i32_lengthed(*y)(
                                dec::be_i32_lengthed(*y)(&mut be_i32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_i32_lengthed(*y)(*x),
                            le_i32_lengthed(*y)(
                                dec::le_i32_lengthed(*y)(&mut le_i32_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_i32_lengthed(*y)(&mut be_i32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_i32_lengthed(*y)(&mut le_i32_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_i64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::i64(&mut i64(*x).as_slice()).unwrap()));
        random_i64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_i64(&mut be_i64(*x).as_slice()).unwrap()));
        random_i64()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_i64(&mut le_i64(*x).as_slice()).unwrap()));
        random_i64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_i64_lengthed(NBYTES_64)(&mut be_i64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i64().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_i64_lengthed(NBYTES_64)(&mut le_i64_lengthed(NBYTES_64)(*x).as_slice())
                    .unwrap()
            )
        });
        random_i64().iter().for_each(|x| {
            random_usize(NBYTES_64 * NBYTES_64).iter().for_each(|y| {
                match *y < NBYTES_64 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_i64_lengthed(*y)(*x),
                            be_i64_lengthed(*y)(
                                dec::be_i64_lengthed(*y)(&mut be_i64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_i64_lengthed(*y)(*x),
                            le_i64_lengthed(*y)(
                                dec::le_i64_lengthed(*y)(&mut le_i64_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_i64_lengthed(*y)(&mut be_i64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_i64_lengthed(*y)(&mut le_i64_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });

        random_i128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::i128(&mut i128(*x).as_slice()).unwrap()));
        random_i128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::be_i128(&mut be_i128(*x).as_slice()).unwrap()));
        random_i128()
            .iter()
            .for_each(|x| assert_eq!(x, &dec::le_i128(&mut le_i128(*x).as_slice()).unwrap()));
        random_i128().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::be_i128_lengthed(NBYTES_128)(
                    &mut be_i128_lengthed(NBYTES_128)(*x).as_slice()
                )
                .unwrap()
            )
        });
        random_i128().iter().for_each(|x| {
            assert_eq!(
                x,
                &dec::le_i128_lengthed(NBYTES_128)(
                    &mut le_i128_lengthed(NBYTES_128)(*x).as_slice()
                )
                .unwrap()
            )
        });
        random_i128().iter().for_each(|x| {
            random_usize(NBYTES_128 * NBYTES_128).iter().for_each(|y| {
                match *y < NBYTES_128 {
                    // there is information loss here, due to not enough bytes in the input
                    true => {
                        assert_eq!(
                            be_i128_lengthed(*y)(*x),
                            be_i128_lengthed(*y)(
                                dec::be_i128_lengthed(*y)(&mut be_i128_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                        assert_eq!(
                            le_i128_lengthed(*y)(*x),
                            le_i128_lengthed(*y)(
                                dec::le_i128_lengthed(*y)(&mut le_i128_lengthed(*y)(*x).as_slice())
                                    .unwrap()
                            ),
                        );
                    }
                    // no information loss: output is either equal or padded with 0's / MaybeUninitialized slice
                    false => {
                        assert_eq!(
                            x,
                            &dec::be_i128_lengthed(*y)(&mut be_i128_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                        assert_eq!(
                            x,
                            &dec::le_i128_lengthed(*y)(&mut le_i128_lengthed(*y)(*x).as_slice())
                                .unwrap()
                        );
                    }
                }
            })
        });
    }
}
