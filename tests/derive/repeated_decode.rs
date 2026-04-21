// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SimpleRecord {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = *tinyklv::enc::binary::be_u16)]
    value: u16,
}

fn build_multi_record(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&v| SimpleRecord { value: v }.encode_value())
        .collect()
}

#[test]
/// Tests that `repeated` on an empty stream yields an empty `Vec` without erroring.
fn repeated_decode_empty_stream_returns_empty_vec() {
    let mut input: &[u8] = &[];
    let results = SimpleRecord::repeated(&mut input).unwrap();
    assert!(results.is_empty());
}

#[test]
/// Verifies that `repeated` extracts a single record from a stream containing exactly one encoded record.
fn repeated_decode_one_record() {
    let data = build_multi_record(&[0x1234]);
    let results = SimpleRecord::repeated(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, 0x1234);
}

#[test]
/// Tests that without sentinel framing `repeated` consumes the entire stream in one decode call, merging duplicate keys with last-wins semantics.
fn repeated_decode_three_unframed_records_merge_last_wins() {
    let data = build_multi_record(&[1, 2, 3]);
    let results = SimpleRecord::repeated(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, 3);
}

#[test]
/// Tests that `repeated` stops cleanly when the stream contains one complete record followed by a truncated tail.
fn repeated_decode_stops_on_malformed_stream() {
    let mut data = build_multi_record(&[10]);
    data.push(0x01); // truncated second record (no len/val)
    let results = SimpleRecord::repeated(&mut data.as_slice()).unwrap();
    assert!(
        !results.is_empty(),
        "should decode at least the one complete record"
    );
}

#[test]
/// Tests that five unframed records sharing one key collapse into a single decoded record with last-wins value under `repeated`.
fn repeated_decode_values_single_key_last_wins() {
    let values = [100_u16, 200, 300, 400, 500];
    let data = build_multi_record(&values);
    let results = SimpleRecord::repeated(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, 500);
}
