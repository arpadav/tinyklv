use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SR",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SimpleRecord {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

fn build_multi_record(values: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    for &v in values {
        SimpleRecord { value: v }.encode_frame(&mut out);
    }
    out
}

#[test]
/// Tests that `drain_frames` on an empty stream yields an empty `Vec` without erroring.
fn drain_frames_empty_stream_returns_empty_vec() {
    let mut input: &[u8] = &[];
    let results = SimpleRecord::drain_frames(&mut input).unwrap();
    assert!(results.is_empty());
}

#[test]
/// Verifies that `drain_frames` extracts a single record from a stream containing exactly one encoded frame.
fn drain_frames_one_record() {
    let data = build_multi_record(&[0x1234]);
    let results = SimpleRecord::drain_frames(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, 0x1234);
}

#[test]
/// Tests that sentinel framing lets `drain_frames` extract three independent frames.
fn drain_frames_three_framed_records() {
    let data = build_multi_record(&[1, 2, 3]);
    let results = SimpleRecord::drain_frames(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].value, 1);
    assert_eq!(results[1].value, 2);
    assert_eq!(results[2].value, 3);
}

#[test]
/// Tests that `drain_frames` stops cleanly when the stream contains one complete frame followed by a truncated tail.
fn drain_frames_stops_on_malformed_tail() {
    let mut data = build_multi_record(&[10]);
    data.push(0x01);
    let results = SimpleRecord::drain_frames(&mut data.as_slice()).unwrap();
    assert!(
        !results.is_empty(),
        "should decode at least the one complete record"
    );
}

#[test]
/// Tests that five sentinel-framed records are each decoded independently.
fn drain_frames_five_framed_records() {
    let values = [100_u16, 200, 300, 400, 500];
    let data = build_multi_record(&values);
    let results = SimpleRecord::drain_frames(&mut data.as_slice()).unwrap();
    assert_eq!(results.len(), 5);
    for (got, want) in results.iter().zip(values.iter()) {
        assert_eq!(got.value, *want);
    }
}
