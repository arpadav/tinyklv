//! Binary decode codecs for KLV data
//!
//! Provides parsers for converting raw byte slices into Rust primitive types
//! and variable-length integer encodings. All parsers are compatible with the
//! [`winnow`] streaming parser framework and accept `&mut &[u8]` input
//!
//! Includes:
//! * Big-endian, little-endian, and native-endian numeric decoders for all
//!   standard integer and float types
//! * Variable-length ("lengthed") decoders that handle under/over-length
//!   byte slices via truncation or zero-padding
//! * `usize` wrapper variants for length-field parsing
//!
//! String decoders live in [`crate::codecs::string::dec`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::token::take;

macro_rules! wrap {
    // narrow integer decoders (<= 16 bits): unchecked cast wins; wider integers and floats use the
    // `(checked ..)` arm where the checked `try_from` length check is unreachable after `take(N)`
    ($ty:ty) => { pastey::paste! {
        #[inline] // not always inline: take(N)+from_be_bytes folds to a load+bswap; the optimizer inlines it regardless, so forcing it only risks bloat in downstream callers
        #[doc = concat!(" Big-endian decoder for `", stringify!($ty), "`: takes `size_of::<", stringify!($ty), ">()` bytes and `from_be_bytes`")]
        #[doc = ""]
        #[doc = " A single sized read + `from_be_bytes` (one bounds check, one load + byte-swap), rather"]
        #[doc = " than winnow's runtime-bounded shift/add integer loop. On insufficient input it returns the"]
        #[doc = " same `take`-style backtrack error and leaves the cursor unadvanced"]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_be_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::be_", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<be_ $ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            const N: usize = ::core::mem::size_of::<$ty>();
            let bytes = take(N).parse_next(input)?;
            // SAFETY: `take(N)` yielded a slice of exactly `N` bytes, so reading `[u8; N]` from its
            // start is in-bounds; `u8` has alignment 1, so the pointer is always aligned for `[u8; N]`.
            let array = unsafe { *bytes.as_ptr().cast::<[u8; N]>() };
            Ok($ty::from_be_bytes(array))
        }

        #[inline] // not always inline: take(N)+from_le_bytes folds to a load; the optimizer inlines it regardless, so forcing it only risks bloat in downstream callers
        #[doc = concat!(" Little-endian decoder for `", stringify!($ty), "`: takes `size_of::<", stringify!($ty), ">()` bytes and `from_le_bytes`")]
        #[doc = ""]
        #[doc = " A single sized read + `from_le_bytes` (one bounds check, one load), rather than winnow's"]
        #[doc = " runtime-bounded shift/add integer loop. On insufficient input it returns the same"]
        #[doc = " `take`-style backtrack error and leaves the cursor unadvanced"]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_le_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::le_", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<le_ $ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            const N: usize = ::core::mem::size_of::<$ty>();
            let bytes = take(N).parse_next(input)?;
            // SAFETY: as for `be_` above - `take(N)` guarantees exactly `N` bytes and `u8` is align-1.
            let array = unsafe { *bytes.as_ptr().cast::<[u8; N]>() };
            Ok($ty::from_le_bytes(array))
        }
    }};

    // wide integer (> 16 bits) and float decoders: checked `try_from`; length check is unreachable
    // after `take(N)`, so LLVM folds it to a bare load
    (checked $ty:ty) => { pastey::paste! {
        #[inline] // not always inline: take(N)+from_be_bytes folds to a load+bswap; the optimizer inlines it regardless, so forcing it only risks bloat in downstream callers
        #[doc = concat!(" Big-endian decoder for `", stringify!($ty), "`: takes `size_of::<", stringify!($ty), ">()` bytes and `from_be_bytes`")]
        #[doc = ""]
        #[doc = " A single sized read + `from_be_bytes` (one bounds check, one load + byte-swap), rather"]
        #[doc = " than winnow's runtime-bounded shift/add integer loop. On insufficient input it returns the"]
        #[doc = " same `take`-style backtrack error and leaves the cursor unadvanced"]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_be_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::be_", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<be_ $ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            const N: usize = ::core::mem::size_of::<$ty>();
            let bytes = take(N).parse_next(input)?;
            let array = <[u8; N]>::try_from(bytes).expect("take(N) yields exactly N bytes");
            Ok($ty::from_be_bytes(array))
        }

        #[inline] // not always inline: take(N)+from_le_bytes folds to a load; the optimizer inlines it regardless, so forcing it only risks bloat in downstream callers
        #[doc = concat!(" Little-endian decoder for `", stringify!($ty), "`: takes `size_of::<", stringify!($ty), ">()` bytes and `from_le_bytes`")]
        #[doc = ""]
        #[doc = " A single sized read + `from_le_bytes` (one bounds check, one load), rather than winnow's"]
        #[doc = " runtime-bounded shift/add integer loop. On insufficient input it returns the same"]
        #[doc = " `take`-style backtrack error and leaves the cursor unadvanced"]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_le_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::le_", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<le_ $ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            const N: usize = ::core::mem::size_of::<$ty>();
            let bytes = take(N).parse_next(input)?;
            let array = <[u8; N]>::try_from(bytes).expect("take(N) yields exactly N bytes");
            Ok($ty::from_le_bytes(array))
        }
    }};
}
macro_rules! wrap_native {
    ($ty:ty) => { pastey::paste! {
        #[inline] // not always inline: thin winnow::binary delegation; the optimizer inlines the small wrapper, no need to force it across crate boundaries
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_ne_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            winnow::binary::$ty(winnow::binary::Endianness::Native).parse_next(input)
        }
    }};

    (simple $ty:ty) => { pastey::paste! {
        #[inline] // not always inline: thin winnow::binary delegation; the optimizer inlines the small wrapper, no need to force it across crate boundaries
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($ty), ").to_ne_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::", stringify!($ty), "(&mut input);")]
        #[doc = concat!(" assert_eq!(result, Ok(1 as ", stringify!($ty), "));")]
        #[doc = " ```"]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::Result<$ty> {
            winnow::binary::$ty.parse_next(input)
        }
    }}
}
wrap!(u8);
wrap_native!(simple u8);
wrap!(u16);
wrap_native!(u16);
wrap!(checked u32);
wrap_native!(u32);
wrap!(checked u64);
wrap_native!(u64);
wrap!(checked u128);
wrap_native!(u128);
wrap!(i8);
wrap_native!(simple i8);
wrap!(i16);
wrap_native!(i16);
wrap!(checked i32);
wrap_native!(i32);
wrap!(checked i64);
wrap_native!(i64);
wrap!(checked i128);
wrap_native!(i128);
wrap!(checked f32);
wrap_native!(f32);
wrap!(checked f64);
wrap_native!(f64);

macro_rules! as_usize {
    ($parser:ident) => { pastey::paste! {
        #[inline] // not always inline: trivial `.map(|v| v as usize)` over a leaf decoder; the optimizer inlines it
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`", stringify!($parser), "()`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($parser), ").to_ne_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::", stringify!($parser), "_as_usize(&mut input);")]
        #[doc = " assert_eq!(result, Ok(1_usize));"]
        #[doc = " ```"]
        pub fn [<$parser _as_usize>](input: &mut &[u8]) -> winnow::Result<usize> {
            $parser(input).map(|val| val as usize)
        }

        #[inline] // not always inline: trivial `.map(|v| v as usize)` over a leaf decoder; the optimizer inlines it
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::be_", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`be_", stringify!($parser), "`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($parser), ").to_be_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::be_", stringify!($parser), "_as_usize(&mut input);")]
        #[doc = " assert_eq!(result, Ok(1_usize));"]
        #[doc = " ```"]
        pub fn [<be_ $parser _as_usize>](input: &mut &[u8]) -> winnow::Result<usize> {
            [<be_ $parser>](input).map(|val| val as usize)
        }

        #[inline] // not always inline: trivial `.map(|v| v as usize)` over a leaf decoder; the optimizer inlines it
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::le_", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`le_", stringify!($parser), "`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
        #[doc = ""]
        #[doc = " # Example"]
        #[doc = ""]
        #[doc = " ```"]
        #[doc = concat!(" let mut input: &[u8] = &(1 as ", stringify!($parser), ").to_le_bytes();")]
        #[doc = concat!(" let result = tinyklv::codecs::binary::dec::le_", stringify!($parser), "_as_usize(&mut input);")]
        #[doc = " assert_eq!(result, Ok(1_usize));"]
        #[doc = " ```"]
        pub fn [<le_ $parser _as_usize>](input: &mut &[u8]) -> winnow::Result<usize> {
            [<le_ $parser>](input).map(|val| val as usize)
        }
    }};
}
as_usize!(u8);
as_usize!(u16);
as_usize!(u32);
as_usize!(u64);
as_usize!(u128);

macro_rules! lengthed_be {
    ($type:ty, $len:expr, $doc:literal) => { pastey::paste! {
        #[inline] // not always inline: returns a closure whose body branches and copies into a pad buffer; let the cost model decide (in-crate LTO inlines it regardless)
        #[allow(
            clippy::indexing_slicing,
            reason = "both index arms are bounds-gated by `if len > $len`: when true, \
                      value.len() == len > $len so value[value.len() - $len..] is in-bounds; when \
                      false, $len - len >= 0 and pval is a [u8; $len] array, so pval[($len - len)..] \
                      (length len) is in-bounds"
        )]
        #[doc = $doc]
        pub fn [<be_ $type _lengthed>](len: usize) -> impl Fn(&mut &[u8]) -> winnow::Result<$type> {
            move |input| {
                let value = take(len).parse_next(input)?;
                if len > $len {
                    let mut value = &value[value.len() - $len..];
                    [<be_ $type>](&mut value)
                } else {
                    let mut pval = [0u8; $len];
                    pval[($len - len)..].copy_from_slice(value);
                    Ok($type::from_be_bytes(pval))
                }
            }
        }
    }};
    ($type:ty, $len:expr) => { lengthed_be!($type, $len, ""); };
}

macro_rules! lengthed_le {
    ($type:ty, $precision_len:expr, $doc:literal) => { pastey::paste! {
        #[inline] // not always inline: returns a closure whose body branches and copies into a pad buffer; let the cost model decide (in-crate LTO inlines it regardless)
        #[allow(
            clippy::indexing_slicing,
            reason = "both index arms are bounds-gated by `if len > $precision_len`: when true, \
                      value.len() == len > $precision_len so value[..$precision_len] is in-bounds; \
                      when false, len <= $precision_len and pval is a [u8; $precision_len] array, so \
                      pval[..len] is in-bounds"
        )]
        #[doc = $doc]
        pub fn [<le_ $type _lengthed>](len: usize) -> impl Fn(&mut &[u8]) -> winnow::Result<$type> {
            move |input| {
                let value = take(len).parse_next(input)?;
                if len > $precision_len {
                    let mut value = &value[..$precision_len];
                    [<le_ $type>](&mut value)
                } else {
                    let mut pval = [0u8; $precision_len];
                    pval[..len].copy_from_slice(value);
                    Ok($type::from_le_bytes(pval))
                }
            }
        }
    }};
    ($type:ty, $len:expr) => { lengthed_le!($type, $len, ""); };
}

lengthed_be!(
    u8,
    1,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u8`] value
using big-endian encoding.

* len 3: [SKIP, SKIP, VAL] -> [VAL]
* len 1: [VAL] -> [VAL]

# Example

```rust
let mut input1: &[u8] = &[0xAA, 0xBB, 0xCC];
let mut input2: &[u8] = &[0x42];
assert_eq!(tinyklv::dec::binary::be_u8_lengthed(3)(&mut input1), Ok(0xCC));
assert_eq!(tinyklv::dec::binary::be_u8_lengthed(1)(&mut input2), Ok(0x42));
```
"
);
lengthed_be!(
    u16,
    2,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u16`] value
using big-endian encoding.

* len 8: [SKIP, SKIP, SKIP, SKIP, SKIP, SKIP, VAL, VAL] -> [VAL, VAL]
* len 1: [VAL] -> [0x00, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x01, 0xE0, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01, 0xE0];
let mut input3: &[u8] = &[0xE0];
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(4)(&mut input1), Ok(0xFFFF));
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(2)(&mut input2), Ok(0x01E0));
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(1)(&mut input3), Ok(0x00E0));
```
"
);
lengthed_be!(
    u32,
    4,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u32`] value
using big-endian encoding.

* len 8: [SKIP, SKIP, SKIP, SKIP, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [0x00, 0x00, 0x00, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x00, 0x01, 0xE0, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x00, 0x01, 0xE0, 0xFF];
let mut input3: &[u8] = &[0x01, 0xE0, 0xFF];
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(5)(&mut input1), Ok(0x01E0FFFF));
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(4)(&mut input2), Ok(0x0001E0FF));
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(3)(&mut input3), Ok(0x0001E0FF));
```
"
);
lengthed_be!(u64, 8, "
Converts a [`prim@u8`] slice of any length into a [`prim@u64`] value
using big-endian encoding.

* len 10: [SKIP, SKIP, VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, VAL]
* len 5: [VAL, VAL, VAL, VAL, VAL] -> [0x00, 0x00, 0x00, VAL, VAL, VAL, VAL, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
let mut input2: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
let mut input3: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(9)(&mut input1), Ok(0x00_01_E0_FF_FF_00_00_00));
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(8)(&mut input2), Ok(0x00_00_01_E0_FF_FF_00_00));
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(7)(&mut input3), Ok(0x00_00_00_01_E0_FF_FF_00));
```
");
lengthed_be!(
    u128,
    16,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u128`] value
using big-endian encoding.

* len > 16: takes last 16 bytes
* len < 16: zero-pads high bytes

# Example

```rust
let mut input: &[u8] = &[0x01, 0x02];
assert_eq!(tinyklv::dec::binary::be_u128_lengthed(2)(&mut input), Ok(0x0102_u128));
```
"
);
lengthed_be!(
    i8,
    1,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i8`] value
using big-endian encoding.

* len 3: [SKIP, SKIP, VAL] -> [VAL]
* len 1: [VAL] -> [VAL]

# Example

```rust
let mut input: &[u8] = &[0xFF];
assert_eq!(tinyklv::dec::binary::be_i8_lengthed(1)(&mut input), Ok(-1_i8));
```
"
);
lengthed_be!(
    i16,
    2,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i16`] value
using big-endian encoding.

* len 4: [SKIP, SKIP, VAL, VAL] -> [VAL, VAL]
* len 1: [VAL] -> [0x00, VAL]

# Example

```rust
let mut input: &[u8] = &[0xFF, 0xFE];
assert_eq!(tinyklv::dec::binary::be_i16_lengthed(2)(&mut input), Ok(-2_i16));
```
"
);
lengthed_be!(
    i32,
    4,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i32`] value
using big-endian encoding.

* len 8: [SKIP, SKIP, SKIP, SKIP, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL]
* len 2: [VAL, VAL] -> [0x00, 0x00, VAL, VAL]

# Example

```rust
let mut input: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
assert_eq!(tinyklv::dec::binary::be_i32_lengthed(4)(&mut input), Ok(-1_i32));
```
"
);
lengthed_be!(
    i64,
    8,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i64`] value
using big-endian encoding.

* len 10: takes last 8 bytes
* len 4: [VAL, VAL, VAL, VAL] -> [0x00, 0x00, 0x00, 0x00, VAL, VAL, VAL, VAL]

# Example

```rust
let mut input: &[u8] = &[0xFF; 8];
assert_eq!(tinyklv::dec::binary::be_i64_lengthed(8)(&mut input), Ok(-1_i64));
```
"
);
lengthed_be!(
    i128,
    16,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i128`] value
using big-endian encoding.

* len > 16: takes last 16 bytes
* len < 16: zero-pads high bytes

# Example

```rust
let mut input: &[u8] = &[0xFF; 16];
assert_eq!(tinyklv::dec::binary::be_i128_lengthed(16)(&mut input), Ok(-1_i128));
```
"
);
lengthed_le!(
    u8,
    1,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u8`] value
using little-endian encoding.

* len 3: [VAL, SKIP, SKIP] -> [VAL]
* len 1: [VAL] -> [VAL]

# Example

```rust
let mut input: &[u8] = &[0xCC, 0xBB, 0xAA];
assert_eq!(tinyklv::dec::binary::le_u8_lengthed(3)(&mut input), Ok(0xCC));
```
"
);
lengthed_le!(
    u16,
    2,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u16`] value
using little-endian encoding.

* len 8: [VAL, VAL, SKIP, SKIP, SKIP, SKIP, SKIP, SKIP] -> [VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01];
let num1 = tinyklv::dec::binary::le_u16_lengthed(5)(&mut input1);
let num2 = tinyklv::dec::binary::le_u16_lengthed(1)(&mut input2);
assert_eq!(num1, Ok(480));
assert_eq!(num2, Ok(1));
```
"
);
lengthed_le!(
    u32,
    4,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u32`] value
using little-endian encoding.

* len 8: [VAL, VAL, VAL, VAL, SKIP, SKIP, SKIP, SKIP] -> [VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00, 0x00, 0x00]
* len 3: [VAL, VAL, VAL] -> [VAL, VAL, VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01];
let mut input3: &[u8] = &[0x01, 0x02, 0x03];
let num1 = tinyklv::dec::binary::le_u32_lengthed(5)(&mut input1);
let num2 = tinyklv::dec::binary::le_u32_lengthed(1)(&mut input2);
let num3 = tinyklv::dec::binary::le_u32_lengthed(3)(&mut input3);
assert_eq!(num1, Ok(4_294_902_240));
assert_eq!(num2, Ok(1));
assert_eq!(num3, Ok(197_121));
```
"
);
lengthed_le!(u64, 8, "
Converts a [`prim@u8`] slice of any length into a [`prim@u64`] value
using little-endian encoding.

* len 8: [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
* len 7: [VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
let mut input2: &[u8] = &[0x01];
let mut input3: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
let num1 = tinyklv::dec::binary::le_u64_lengthed(16)(&mut input1);
let num2 = tinyklv::dec::binary::le_u64_lengthed(1)(&mut input2);
let num3 = tinyklv::dec::binary::le_u64_lengthed(7)(&mut input3);
assert_eq!(num1, Ok(1_099_511_562_720));
assert_eq!(num2, Ok(1));
assert_eq!(num3, Ok(1_976_943_448_883_713));
```
");
lengthed_le!(
    u128,
    16,
    "
Converts a [`prim@u8`] slice of any length into a [`prim@u128`] value
using little-endian encoding.

* len > 16: takes first 16 bytes
* len < 16: zero-pads high bytes

# Example

```rust
let mut input: &[u8] = &[0x01, 0x02];
assert_eq!(tinyklv::dec::binary::le_u128_lengthed(2)(&mut input), Ok(0x0201_u128));
```
"
);
lengthed_le!(
    i8,
    1,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i8`] value
using little-endian encoding.

* len 3: [VAL, SKIP, SKIP] -> [VAL]
* len 1: [VAL] -> [VAL]

# Example

```rust
let mut input: &[u8] = &[0xFF];
assert_eq!(tinyklv::dec::binary::le_i8_lengthed(1)(&mut input), Ok(-1_i8));
```
"
);
lengthed_le!(
    i16,
    2,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i16`] value
using little-endian encoding.

* len 4: [VAL, VAL, SKIP, SKIP] -> [VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00]

# Example

```rust
let mut input: &[u8] = &[0xFE, 0xFF];
assert_eq!(tinyklv::dec::binary::le_i16_lengthed(2)(&mut input), Ok(-2_i16));
```
"
);
lengthed_le!(
    i32,
    4,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i32`] value
using little-endian encoding.

* len 8: [VAL, VAL, VAL, VAL, SKIP, SKIP, SKIP, SKIP] -> [VAL, VAL, VAL, VAL]
* len 2: [VAL, VAL] -> [VAL, VAL, 0x00, 0x00]

# Example

```rust
let mut input: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
assert_eq!(tinyklv::dec::binary::le_i32_lengthed(4)(&mut input), Ok(-1_i32));
```
"
);
lengthed_le!(
    i64,
    8,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i64`] value
using little-endian encoding.

* len 10: takes first 8 bytes
* len 4: [VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, 0x00, 0x00, 0x00, 0x00]

# Example

```rust
let mut input: &[u8] = &[0xFF; 8];
assert_eq!(tinyklv::dec::binary::le_i64_lengthed(8)(&mut input), Ok(-1_i64));
```
"
);
lengthed_le!(
    i128,
    16,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@i128`] value
using little-endian encoding.

* len > 16: takes first 16 bytes
* len < 16: zero-pads high bytes

# Example

```rust
let mut input: &[u8] = &[0xFF; 16];
assert_eq!(tinyklv::dec::binary::le_i128_lengthed(16)(&mut input), Ok(-1_i128));
```
"
);

lengthed_be!(
    f32,
    4,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@f32`] value
using big-endian encoding.

* len > 4: takes last 4 bytes
* len < 4: zero-pads high bytes

**Note:** at a non-native `len` this reinterprets bytes, not precision - truncating or zero-padding an
IEEE-754 float's bytes yields a different value, not a scaled one. It stays byte-symmetric with
[`crate::codecs::binary::enc::be_f32_lengthed`], so encode-decode-encode is byte-stable, but the
decoded value equals the original only when `len` is the native width (4).

# Example

```rust
let mut input: &[u8] = &[0x3F, 0x80, 0x00, 0x00]; // 1.0_f32 in BE
assert_eq!(tinyklv::dec::binary::be_f32_lengthed(4)(&mut input), Ok(1.0_f32));
```
"
);
lengthed_be!(
    f64,
    8,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@f64`] value
using big-endian encoding.

* len > 8: takes last 8 bytes
* len < 8: zero-pads high bytes

**Note:** at a non-native `len` this reinterprets bytes, not precision - truncating or zero-padding an
IEEE-754 float's bytes yields a different value, not a scaled one. It stays byte-symmetric with
[`crate::codecs::binary::enc::be_f64_lengthed`], so encode-decode-encode is byte-stable, but the
decoded value equals the original only when `len` is the native width (8).

# Example

```rust
let mut input: &[u8] = &[0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // 1.0_f64 in BE
assert_eq!(tinyklv::dec::binary::be_f64_lengthed(8)(&mut input), Ok(1.0_f64));
```
"
);
lengthed_le!(
    f32,
    4,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@f32`] value
using little-endian encoding.

* len > 4: takes first 4 bytes
* len < 4: zero-pads high bytes

**Note:** at a non-native `len` this reinterprets bytes, not precision - truncating or zero-padding an
IEEE-754 float's bytes yields a different value, not a scaled one. It stays byte-symmetric with
[`crate::codecs::binary::enc::le_f32_lengthed`], so encode-decode-encode is byte-stable, but the
decoded value equals the original only when `len` is the native width (4).

# Example

```rust
let mut input: &[u8] = &[0x00, 0x00, 0x80, 0x3F]; // 1.0_f32 in LE
assert_eq!(tinyklv::dec::binary::le_f32_lengthed(4)(&mut input), Ok(1.0_f32));
```
"
);
lengthed_le!(
    f64,
    8,
    "
Converts a [`prim@u8`] slice of any length into an [`prim@f64`] value
using little-endian encoding.

* len > 8: takes first 8 bytes
* len < 8: zero-pads high bytes

**Note:** at a non-native `len` this reinterprets bytes, not precision - truncating or zero-padding an
IEEE-754 float's bytes yields a different value, not a scaled one. It stays byte-symmetric with
[`crate::codecs::binary::enc::le_f64_lengthed`], so encode-decode-encode is byte-stable, but the
decoded value equals the original only when `len` is the native width (8).

# Example

```rust
let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F]; // 1.0_f64 in LE
assert_eq!(tinyklv::dec::binary::le_f64_lengthed(8)(&mut input), Ok(1.0_f64));
```
"
);
