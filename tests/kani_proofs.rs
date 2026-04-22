#[cfg(kani)]
mod proofs {
    use tinyklv::codecs::ber::{BerLength, BerOid};
    use tinyklv::prelude::*;

    /// Proof: `BerLength` encode/decode roundtrip holds for every u8 value.
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_roundtrip_u8() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u128, decoded.as_u128());
    }

    /// Proof: `BerLength` encode/decode roundtrip holds for every u16 value.
    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_length_roundtrip_u16() {
        let val: u16 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u128, decoded.as_u128());
    }

    /// Proof: `BerLength::encode_value` never produces an empty byte sequence for any u8.
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_never_empty() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert!(!encoded.is_empty());
    }

    /// Proof: for all `val < 128`, short-form encoding is exactly one byte equal to the value.
    #[kani::proof]
    #[kani::unwind(4)]
    fn ber_length_short_form_single_byte() {
        let val: u8 = kani::any();
        kani::assume(val < 128);
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], val);
    }

    /// Proof: for all `val >= 128`, the long-form first byte has MSB set.
    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_length_long_form_msb_set() {
        let val: u16 = kani::any();
        kani::assume(val >= 128);
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert!(encoded[0] & 0x80 != 0);
    }

    /// Proof: `BerLength::decode` consumes exactly the number of bytes produced by `encode_value` (no leftovers).
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_decode_consumes_all() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerLength::<u64>::decode(&mut slice).unwrap();
        assert!(slice.is_empty());
    }

    /// Proof: `BerOid` encode/decode roundtrip holds for every non-zero u8.
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_roundtrip_u8_nonzero() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let decoded = BerOid::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u64, decoded.value);
    }

    /// Proof: `BerOid` encode/decode roundtrip holds for every non-zero u16.
    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_oid_roundtrip_u16_nonzero() {
        let val: u16 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let decoded = BerOid::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u64, decoded.value);
    }

    /// Proof: the final byte of any `BerOid` encoding (for non-zero u8) has MSB clear.
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_final_byte_msb_clear() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let last = encoded[encoded.len() - 1];
        assert_eq!(last & 0x80, 0);
    }

    /// Proof: all non-final bytes of a multi-byte `BerOid` encoding have MSB set (continuation bit).
    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_oid_continuation_bytes_msb_set() {
        let val: u16 = kani::any();
        kani::assume(val >= 128); // multi-byte encoding
        let encoded = BerOid::new(&(val as u64)).encode_value();
        for i in 0..encoded.len() - 1 {
            assert!(encoded[i] & 0x80 != 0);
        }
    }

    /// Proof: `BerOid` encoding of any non-zero u8 is never empty.
    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_encoding_never_empty_nonzero() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        assert!(!encoded.is_empty());
    }

    /// Proof: `u8` encode/`u8` decode roundtrip holds for every u8.
    #[kani::proof]
    #[kani::unwind(4)]
    fn u8_roundtrip() {
        let val: u8 = kani::any();
        let encoded = tinyklv::enc::binary::u8(val);
        let decoded = tinyklv::dec::binary::u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `i8` encode/`i8` decode roundtrip holds for every i8.
    #[kani::proof]
    #[kani::unwind(4)]
    fn i8_roundtrip() {
        let val: i8 = kani::any();
        let encoded = tinyklv::enc::binary::i8(val);
        let decoded = tinyklv::dec::binary::i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `be_u16` encode/decode roundtrip holds for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn be_u16_roundtrip() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        let decoded = tinyklv::dec::binary::be_u16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `be_i16` encode/decode roundtrip holds for every i16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn be_i16_roundtrip() {
        let val: i16 = kani::any();
        let encoded = tinyklv::enc::binary::be_i16(val);
        let decoded = tinyklv::dec::binary::be_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `be_u32` encode/decode roundtrip holds for every u32.
    #[kani::proof]
    #[kani::unwind(12)]
    fn be_u32_roundtrip() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::be_u32(val);
        let decoded = tinyklv::dec::binary::be_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `le_u16` encode/decode roundtrip holds for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn le_u16_roundtrip() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::le_u16(val);
        let decoded = tinyklv::dec::binary::le_u16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `le_i16` encode/decode roundtrip holds for every i16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn le_i16_roundtrip() {
        let val: i16 = kani::any();
        let encoded = tinyklv::enc::binary::le_i16(val);
        let decoded = tinyklv::dec::binary::le_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `le_u32` encode/decode roundtrip holds for every u32.
    #[kani::proof]
    #[kani::unwind(12)]
    fn le_u32_roundtrip() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::le_u32(val);
        let decoded = tinyklv::dec::binary::le_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: BE and LE u16 encodings have equal length for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn be_le_same_length_u16() {
        let val: u16 = kani::any();
        let be = tinyklv::enc::binary::be_u16(val);
        let le = tinyklv::enc::binary::le_u16(val);
        assert_eq!(be.len(), le.len());
    }

    /// Proof: `be_u16` encoding length equals `size_of::<u16>()` for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn encoding_length_equals_sizeof_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        assert_eq!(encoded.len(), std::mem::size_of::<u16>());
    }

    /// Proof: `be_u32` encoding length equals `size_of::<u32>()` for every u32.
    #[kani::proof]
    #[kani::unwind(12)]
    fn encoding_length_equals_sizeof_u32() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::be_u32(val);
        assert_eq!(encoded.len(), std::mem::size_of::<u32>());
    }

    /// Proof: `be_u16` output matches `u16::to_be_bytes` for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn be_matches_to_be_bytes_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

    /// Proof: `le_u16` output matches `u16::to_le_bytes` for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn le_matches_to_le_bytes_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::le_u16(val);
        assert_eq!(encoded, val.to_le_bytes().to_vec());
    }

    /// Proof: `be_u16_lengthed(len)` output has exactly `len` bytes for every u16 and `len` in `1..=4`.
    #[kani::proof]
    #[kani::unwind(12)]
    fn be_lengthed_output_length_u16() {
        let val: u16 = kani::any();
        let len: usize = kani::any();
        kani::assume(len >= 1 && len <= 4);
        let encoded = tinyklv::enc::binary::be_u16_lengthed(len)(val);
        assert_eq!(encoded.len(), len);
    }

    /// Proof: `le_u16_lengthed(len)` output has exactly `len` bytes for every u16 and `len` in `1..=4`.
    #[kani::proof]
    #[kani::unwind(12)]
    fn le_lengthed_output_length_u16() {
        let val: u16 = kani::any();
        let len: usize = kani::any();
        kani::assume(len >= 1 && len <= 4);
        let encoded = tinyklv::enc::binary::le_u16_lengthed(len)(val);
        assert_eq!(encoded.len(), len);
    }

    /// Proof: `be_u16_lengthed(2)` encode/decode roundtrip holds for every u16.
    #[kani::proof]
    #[kani::unwind(8)]
    fn lengthed_roundtrip_exact_size_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16_lengthed(2)(val);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    /// Proof: `be_u16_from_usize` matches `be_u16` directly for every u16 value cast to usize.
    #[kani::proof]
    #[kani::unwind(8)]
    fn from_usize_matches_direct_u16() {
        let val: u16 = kani::any();
        let direct = tinyklv::enc::binary::be_u16(val);
        let from_usize = tinyklv::enc::binary::be_u16_from_usize(val as usize);
        assert_eq!(direct, from_usize);
    }
}
