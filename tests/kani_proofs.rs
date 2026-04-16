#[cfg(kani)]
mod proofs {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::codecs::ber::{BerLength, BerOid};
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // ber length proofs
    // --------------------------------------------------

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_roundtrip_u8() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u128, decoded.as_u128());
    }

    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_length_roundtrip_u16() {
        let val: u16 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u128, decoded.as_u128());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_never_empty() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert!(!encoded.is_empty());
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn ber_length_short_form_single_byte() {
        let val: u8 = kani::any();
        kani::assume(val < 128);
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], val);
    }

    #[kani::proof]
    #[kani::unwind(12)]
    #[kani::solver(cadical)]
    fn ber_length_long_form_msb_set() {
        let val: u16 = kani::any();
        kani::assume(val >= 128);
        let encoded = BerLength::new(&(val as u64)).encode_value();
        assert!(encoded[0] & 0x80 != 0);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_length_decode_consumes_all() {
        let val: u8 = kani::any();
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerLength::<u64>::decode(&mut slice).unwrap();
        assert!(slice.is_empty());
    }

    // --------------------------------------------------
    // ber oid proofs
    // --------------------------------------------------

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_roundtrip_u8_nonzero() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let decoded = BerOid::<u64>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(val as u64, decoded.value);
    }

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

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_final_byte_msb_clear() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let last = encoded[encoded.len() - 1];
        assert_eq!(last & 0x80, 0);
    }

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

    #[kani::proof]
    #[kani::unwind(8)]
    fn ber_oid_encoding_never_empty_nonzero() {
        let val: u8 = kani::any();
        kani::assume(val > 0);
        let encoded = BerOid::new(&(val as u64)).encode_value();
        assert!(!encoded.is_empty());
    }

    // --------------------------------------------------
    // binary codec proofs
    // --------------------------------------------------

    #[kani::proof]
    #[kani::unwind(4)]
    fn be_u8_roundtrip() {
        let val: u8 = kani::any();
        let encoded = tinyklv::enc::binary::u8(val);
        let decoded = tinyklv::dec::binary::be_u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn be_i8_roundtrip() {
        let val: i8 = kani::any();
        let encoded = tinyklv::enc::binary::i8(val);
        let decoded = tinyklv::dec::binary::be_i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn be_u16_roundtrip() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        let decoded = tinyklv::dec::binary::be_u16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn be_i16_roundtrip() {
        let val: i16 = kani::any();
        let encoded = tinyklv::enc::binary::be_i16(val);
        let decoded = tinyklv::dec::binary::be_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn be_u32_roundtrip() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::be_u32(val);
        let decoded = tinyklv::dec::binary::be_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn le_u16_roundtrip() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::le_u16(val);
        let decoded = tinyklv::dec::binary::le_u16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn le_i16_roundtrip() {
        let val: i16 = kani::any();
        let encoded = tinyklv::enc::binary::le_i16(val);
        let decoded = tinyklv::dec::binary::le_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn le_u32_roundtrip() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::le_u32(val);
        let decoded = tinyklv::dec::binary::le_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn be_le_same_length_u16() {
        let val: u16 = kani::any();
        let be = tinyklv::enc::binary::be_u16(val);
        let le = tinyklv::enc::binary::le_u16(val);
        assert_eq!(be.len(), le.len());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn encoding_length_equals_sizeof_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        assert_eq!(encoded.len(), std::mem::size_of::<u16>());
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn encoding_length_equals_sizeof_u32() {
        let val: u32 = kani::any();
        let encoded = tinyklv::enc::binary::be_u32(val);
        assert_eq!(encoded.len(), std::mem::size_of::<u32>());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn be_matches_to_be_bytes_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16(val);
        assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn le_matches_to_le_bytes_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::le_u16(val);
        assert_eq!(encoded, val.to_le_bytes().to_vec());
    }

    // --------------------------------------------------
    // lengthed proofs
    // --------------------------------------------------

    #[kani::proof]
    #[kani::unwind(12)]
    fn be_lengthed_output_length_u16() {
        let val: u16 = kani::any();
        let len: usize = kani::any();
        kani::assume(len >= 1 && len <= 4);
        let encoded = tinyklv::enc::binary::be_u16_lengthed(len)(val);
        assert_eq!(encoded.len(), len);
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn le_lengthed_output_length_u16() {
        let val: u16 = kani::any();
        let len: usize = kani::any();
        kani::assume(len >= 1 && len <= 4);
        let encoded = tinyklv::enc::binary::le_u16_lengthed(len)(val);
        assert_eq!(encoded.len(), len);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn lengthed_roundtrip_exact_size_u16() {
        let val: u16 = kani::any();
        let encoded = tinyklv::enc::binary::be_u16_lengthed(2)(val);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn from_usize_matches_direct_u16() {
        let val: u16 = kani::any();
        let direct = tinyklv::enc::binary::be_u16(val);
        let from_usize = tinyklv::enc::binary::be_u16_from_usize(val as usize);
        assert_eq!(direct, from_usize);
    }
}
