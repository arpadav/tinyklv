//! String encode codecs for KLV data
//!
//! Provides encoders for converting Rust string types and numeric values
//! into raw bytes for KLV value fields. All encoders append into a caller-owned
//! `&mut Vec<u8>` and are compatible with the `#[klv(enc = ...)]` derive macro attribute
//!
//! Includes:
//! * UTF-8 encoder (`from_string_utf8`)
//! * UTF-16 encoders for little-endian and big-endian byte order
//! * ASCII string encoder (`from_string_ascii`)
//! * ASCII-encoded value encoders: base-10 text for all integer and float
//!   primitive types, and base-16 hex text (`hex_*`) for unsigned integers
//!
//! The decode counterparts live in [`crate::codecs::string::dec`]
//!
//! Author: aav

/// Encodes a string as UTF-8 bytes
///
/// **Roundtrip warning**: If the original data was decoded with [`to_string_utf8`](crate::codecs::string::dec::to_string_utf8)
/// (which uses `from_utf8_lossy`), invalid UTF-8 bytes are replaced with U+FFFD
/// during decode. Re-encoding produces different (longer) bytes. Use
/// [`to_string_utf8_strict`](crate::codecs::string::dec::to_string_utf8_strict) on the decode side for lossless roundtrip
///
/// Note: encoding from `&str` is inherently strict (Rust `&str` is always valid
/// UTF-8), so no separate `from_string_utf8_strict` encoder is needed
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf8;
///
/// let mut encoded = Vec::new();
/// from_string_utf8("AF-101", &mut encoded);
/// assert_eq!(encoded, vec![0x41, 0x46, 0x2D, 0x31, 0x30, 0x31]);
/// ```
pub fn from_string_utf8(input: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(input.as_bytes());
}

/// Encodes a string as UTF-16 little-endian bytes
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf16_le;
///
/// let mut encoded = Vec::new();
/// from_string_utf16_le("AB", &mut encoded);
/// // 'A' = 0x0041 LE -> [0x41, 0x00], 'B' = 0x0042 LE -> [0x42, 0x00]
/// assert_eq!(encoded, vec![0x41, 0x00, 0x42, 0x00]);
///
/// // emoji U+1F600 produces a surrogate pair
/// let mut emoji = Vec::new();
/// from_string_utf16_le("\u{1F600}", &mut emoji);
/// assert_eq!(emoji.len(), 4); // 2 code units × 2 bytes
/// ```
pub fn from_string_utf16_le(input: &str, out: &mut Vec<u8>) {
    out.extend(input.encode_utf16().flat_map(u16::to_le_bytes));
}

/// Encodes a string as UTF-16 big-endian bytes
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf16_be;
///
/// let mut encoded = Vec::new();
/// from_string_utf16_be("AB", &mut encoded);
/// // 'A' = 0x0041 BE -> [0x00, 0x41], 'B' = 0x0042 BE -> [0x00, 0x42]
/// assert_eq!(encoded, vec![0x00, 0x41, 0x00, 0x42]);
/// ```
pub fn from_string_utf16_be(input: &str, out: &mut Vec<u8>) {
    out.extend(input.encode_utf16().flat_map(u16::to_be_bytes));
}

/// Equivalent to [`from_string_utf8`] for ASCII input
///
/// Both exist for API symmetry with the decode side, which has separate
/// `to_string_utf8` and `to_string_ascii` functions
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_ascii;
///
/// let mut encoded = Vec::new();
/// from_string_ascii("HELLO", &mut encoded);
/// assert_eq!(encoded, b"HELLO".to_vec());
/// ```
#[cfg(feature = "ascii")]
pub fn from_string_ascii(input: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(input.as_bytes());
}

/// Generates a base-10 integer/float encoder
macro_rules! ascii_display {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Encodes a [`", stringify!($ty), "`] as its base-10 ASCII text.")]
            pub fn [<$ty>](input: $ty, out: &mut Vec<u8>) {
                // write the formatted digits straight into `out`; `Vec<u8>: io::Write`
                // never errors, so the formatting never allocates an intermediate `String`
                use std::io::Write as _;
                let _ = write!(out, "{input}");
            }
        }
    };
}

/// Generates a base-16 unsigned integer encoder
macro_rules! ascii_hex {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Encodes a [`", stringify!($ty), "`] as lower-case base-16 ASCII text.")]
            pub fn [<hex_ $ty>](input: $ty, out: &mut Vec<u8>) {
                // format hex digits straight into `out`; `Vec<u8>: io::Write` never errors,
                // so this never allocates an intermediate `String`
                use std::io::Write as _;
                let _ = write!(out, "{input:x}");
            }
        }
    };
}

ascii_display!(u8);
ascii_display!(u16);
ascii_display!(u32);
ascii_display!(u64);
ascii_display!(u128);

ascii_display!(i8);
ascii_display!(i16);
ascii_display!(i32);
ascii_display!(i64);
ascii_display!(i128);

ascii_display!(f32);
ascii_display!(f64);

ascii_hex!(u8);
ascii_hex!(u16);
ascii_hex!(u32);
ascii_hex!(u64);
ascii_hex!(u128);
