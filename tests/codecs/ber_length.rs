use tinyklv::codecs::ber::BerLength;
use tinyklv::prelude::*;

#[test]
fn short_form_zero() {
    let encoded = tinyklv::enc::ber::ber_length(&0_u8);
    assert_eq!(encoded, vec![0x00]);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 0_usize);
}

#[test]
fn short_form_one() {
    let encoded = tinyklv::enc::ber::ber_length(&1_u8);
    assert_eq!(encoded, vec![0x01]);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 1_usize);
}

#[test]
fn short_form_64() {
    let encoded = tinyklv::enc::ber::ber_length(&64_u8);
    assert_eq!(encoded, vec![0x40]);
    assert!(encoded[0] & 0x80 == 0, "short form MSB must be 0");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 64_usize);
}

#[test]
fn short_form_127() {
    let encoded = tinyklv::enc::ber::ber_length(&127_u8);
    assert_eq!(encoded, vec![0x7F]);
    assert!(encoded[0] & 0x80 == 0, "short form MSB must be 0");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 127_usize);
}

#[test]
fn long_form_128() {
    let encoded = tinyklv::enc::ber::ber_length(&128_u8);
    assert_eq!(encoded.len(), 2);
    assert!(encoded[0] & 0x80 != 0, "long form first byte MSB must be 1");
    assert_eq!(encoded[0] & 0x7F, 1, "should claim 1 extra byte");
    assert_eq!(encoded[1], 128);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 128_usize);
}

#[test]
fn long_form_129() {
    let encoded = tinyklv::enc::ber::ber_length(&129_u8);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 129_usize);
}

#[test]
fn long_form_255() {
    let encoded = tinyklv::enc::ber::ber_length(&255_u8);
    assert!(encoded[0] & 0x80 != 0, "long form first byte MSB must be 1");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 255_usize);
}

#[test]
fn long_form_256() {
    let encoded = tinyklv::enc::ber::ber_length(&256_u16);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 256_usize);
}

#[test]
fn long_form_65535() {
    let encoded = tinyklv::enc::ber::ber_length(&65535_u32);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 65535_usize);
}

#[test]
fn long_form_u32_max() {
    let encoded = tinyklv::enc::ber::ber_length(&u32::MAX);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, u32::MAX as usize);
}

#[test]
fn berlength_struct_47() {
    let val = 47_u64;
    let ber = BerLength::new(&val);
    let encoded = ber.encode_value();
    assert_eq!(encoded, vec![47]);
    let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(&val));
}

#[test]
fn berlength_struct_201() {
    let val = 201_u64;
    let ber = BerLength::new(&val);
    let encoded = ber.encode_value();
    assert_eq!(encoded, vec![128 + 1, 201]);
    let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(&val));
}

#[test]
fn berlength_struct_large() {
    let val = 123891829038102_u64;
    let ber = BerLength::new(&val);
    let encoded = ber.encode_value();
    assert_eq!(encoded, vec![128 + 6, 112, 173, 208, 117, 220, 22]);
    let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(&val));
}

#[test]
fn berlength_8500738_u32() {
    let encoded = BerLength::new(&8_500_738_u32).encode_value();
    assert_eq!(encoded, vec![128 + 3, 129, 182, 2]);
    let decoded = BerLength::<u32>::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(&8_500_738_u32));
}

// --------------------------------------------------
// error cases
// --------------------------------------------------

#[test]
fn ber_length_empty_input_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
fn ber_length_truncated_long_form_fails() {
    // Byte 0x82 claims 2 more bytes, but only 1 follows
    let mut input: &[u8] = &[0x82, 0x01];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
fn ber_length_truncated_long_form_zero_extra() {
    // Byte 0x81 claims 1 more byte, but none follow
    let mut input: &[u8] = &[0x81];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}
