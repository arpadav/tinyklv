/// Encodes a string as UTF-8 bytes.
///
/// **Roundtrip warning**: If the original data was decoded with [`to_string_utf8`](crate::codecs::binary::dec::to_string_utf8)
/// (which uses `from_utf8_lossy`), invalid UTF-8 bytes are replaced with U+FFFD
/// during decode. Re-encoding produces different (longer) bytes. Use
/// [`to_string_utf8_strict`](crate::codecs::binary::dec::to_string_utf8_strict) on the decode side for lossless roundtrip.
///
/// Note: encoding from `&str` is inherently strict (Rust `&str` is always valid
/// UTF-8), so no separate `from_string_utf8_strict` encoder is needed.
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf8;
///
/// let encoded = from_string_utf8("AF-101");
/// assert_eq!(encoded, vec![0x41, 0x46, 0x2D, 0x31, 0x30, 0x31]);
/// ```
pub fn from_string_utf8(input: &str) -> Vec<u8> {
    input.as_bytes().to_vec()
}

/// Encodes a string as UTF-16 little-endian bytes.
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant.
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf16_le;
///
/// let encoded = from_string_utf16_le("AB");
/// // 'A' = 0x0041 LE -> [0x41, 0x00], 'B' = 0x0042 LE -> [0x42, 0x00]
/// assert_eq!(encoded, vec![0x41, 0x00, 0x42, 0x00]);
///
/// // emoji U+1F600 produces a surrogate pair
/// let emoji = from_string_utf16_le("\u{1F600}");
/// assert_eq!(emoji.len(), 4); // 2 code units × 2 bytes
/// ```
pub fn from_string_utf16_le(input: &str) -> Vec<u8> {
    input.encode_utf16().flat_map(|c| c.to_le_bytes()).collect()
}

/// Encodes a string as UTF-16 big-endian bytes.
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant.
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_utf16_be;
///
/// let encoded = from_string_utf16_be("AB");
/// // 'A' = 0x0041 BE -> [0x00, 0x41], 'B' = 0x0042 BE -> [0x00, 0x42]
/// assert_eq!(encoded, vec![0x00, 0x41, 0x00, 0x42]);
/// ```
pub fn from_string_utf16_be(input: &str) -> Vec<u8> {
    input.encode_utf16().flat_map(|c| c.to_be_bytes()).collect()
}

/// Equivalent to [`from_string_utf8`] for ASCII input.
///
/// Both exist for API symmetry with the decode side, which has separate
/// `to_string_utf8` and `to_string_ascii` functions.
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::enc::from_string_ascii;
///
/// let encoded = from_string_ascii("HELLO");
/// assert_eq!(encoded, b"HELLO".to_vec());
/// ```
#[cfg(feature = "ascii")]
pub fn from_string_ascii(input: &str) -> Vec<u8> {
    input.as_bytes().to_vec()
}
