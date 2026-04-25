//! Streaming tests with a realistic multi-type KLV struct.
//!
//! `SensorReport` carries u16, f32, u32, u8, and Option<u16> fields to
//! exercise the `Decoder` API beyond the simple all-u8 test structs used
//! elsewhere.

use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream = &[u8],  // default, shown for clarity
    sentinel = b"SENSOR",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SensorReport {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16
    )]
    node_id: u16,

    #[klv(
        key = 0x02,
        dec = decb::be_f32,
        enc = *encb::be_f32
    )]
    temperature: f32,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32
    )]
    status_code: u32,

    #[klv(
        key = 0x04,
        dec = decb::u8,
        enc = *encb::u8
    )]
    battery_pct: u8,

    #[klv(
        key = 0x05,
        dec = decb::be_u16,
        enc = *encb::be_u16
    )]
    humidity: Option<u16>,
}
impl SensorReport {
    fn new(node_id: u16, temp: f32, status: u32, batt: u8, humidity: Option<u16>) -> SensorReport {
        SensorReport {
            node_id,
            temperature: temp,
            status_code: status,
            battery_pct: batt,
            humidity,
        }
    }
}

#[test]
/// One complete `SensorReport` encoded, fed in a single shot, and decoded.
/// Asserts the decoded value matches exactly, then that the buffer is empty
/// and a second `next()` returns `None`.
fn mixed_one_shot() {
    let pkt = SensorReport::new(1001, 23.5_f32, 0x0000_0001, 87, None);
    let frame = pkt.encode_frame();

    let mut dec = SensorReport::decoder();
    dec.feed(&frame);

    let got = dec.next().expect("expected Ready");
    assert_eq!(got, pkt);

    assert!(dec.next().is_none(), "empty buffer must yield None");
    assert!(dec.buffered().is_empty());
}

#[test]
/// Three reports with varied field values (including Some and None humidity)
/// are encoded into a single blob and fed byte-by-byte. Asserts all three
/// decode in order with correct values. f32 round-trips are verified by bit
/// pattern equality.
fn mixed_byte_by_byte() {
    let pkts = [
        SensorReport::new(10, 18.25_f32, 0xDEAD_BEEF, 100, Some(4500)),
        SensorReport::new(20, -5.0_f32, 0x0000_0000, 42, None),
        SensorReport::new(30, 99.125_f32, 0x1234_5678, 11, Some(9999)),
    ];

    let blob: Vec<u8> = pkts.iter().flat_map(|p| p.encode_frame()).collect();

    let mut dec = SensorReport::decoder();
    let mut got = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        for r in dec.iter() {
            got.push(r);
        }
    }

    assert_eq!(got.len(), pkts.len());
    for (i, expected) in pkts.iter().enumerate() {
        // f32 equality via bit patterns: be_f32 encode/decode is exact
        assert_eq!(got[i].node_id, expected.node_id);
        assert_eq!(got[i].temperature.to_bits(), expected.temperature.to_bits());
        assert_eq!(got[i].status_code, expected.status_code);
        assert_eq!(got[i].battery_pct, expected.battery_pct);
        assert_eq!(got[i].humidity, expected.humidity);
    }
    assert!(dec.buffered().is_empty());
}

#[test]
/// A blob with junk bytes and zeros before the first sentinel is fed all at
/// once. The sentinel seeker must skip the junk and decode both reports
/// cleanly.
fn mixed_with_junk_prefix() {
    let report1 = SensorReport::new(200, 21.0_f32, 0x0000_0002, 75, Some(6000));
    let report2 = SensorReport::new(201, 22.5_f32, 0x0000_0003, 60, None);

    let mut data: Vec<u8> = vec![
        // junk before first sentinel
        0x00, 0x00, 0xFF, 0xDE, 0xAD, 0x00, 0x00,
    ];
    data.extend(report1.encode_frame());
    data.extend(report2.encode_frame());

    let mut dec = SensorReport::decoder();
    dec.feed(&data);

    let got1 = dec.next().expect("expected first Ready");
    assert_eq!(got1, report1);

    let got2 = dec.next().expect("expected second Ready");
    assert_eq!(got2, report2);

    assert!(dec.next().is_none());
    assert!(dec.buffered().is_empty());
}

#[test]
/// Five reports fed through the same rotating chunk-size pattern used in
/// `streaming_decoder.rs`. Asserts no packet is lost, no duplicate emitted,
/// and no buffer residue remains after all chunks are consumed.
fn mixed_irregular_chunks() {
    let pkts = [
        SensorReport::new(1, 0.0_f32, 0x0000_0001, 99, None),
        SensorReport::new(2, 1.5_f32, 0x0000_0002, 88, Some(3000)),
        SensorReport::new(3, -10.0_f32, 0x0000_0003, 77, None),
        SensorReport::new(4, 100.0_f32, 0x0000_0004, 55, Some(8192)),
        SensorReport::new(5, 37.25_f32, 0x0000_0005, 11, Some(1)),
    ];
    let blob: Vec<u8> = pkts.iter().flat_map(|p| p.encode_frame()).collect();
    let chunk_sizes = [1usize, 4, 2, 3, 9, 5, 7, 11, 2, 8, 1, 6, 3];
    let mut dec = SensorReport::decoder();
    let mut got = Vec::new();
    let mut cursor = 0usize;
    let mut chunk_i = 0usize;
    while cursor < blob.len() {
        let want = chunk_sizes[chunk_i % chunk_sizes.len()];
        let end = (cursor + want).min(blob.len());
        dec.feed(&blob[cursor..end]);
        cursor = end;
        chunk_i += 1;
        for r in dec.iter() {
            got.push(r);
        }
    }

    assert_eq!(got.len(), pkts.len());
    for (i, expected) in pkts.iter().enumerate() {
        assert_eq!(got[i].node_id, expected.node_id);
        assert_eq!(got[i].temperature.to_bits(), expected.temperature.to_bits());
        assert_eq!(got[i].status_code, expected.status_code);
        assert_eq!(got[i].battery_pct, expected.battery_pct);
        assert_eq!(got[i].humidity, expected.humidity);
    }
    assert!(dec.buffered().is_empty());
}

#[test]
/// Two reports streamed byte-by-byte: the first carries `humidity: Some(6500)`,
/// the second carries `humidity: None`. Verifies that the optional field is
/// correctly present in one and absent in the other.
fn mixed_optional_absent() {
    let with_humidity = SensorReport::new(50, 25.0_f32, 0x0000_00AA, 90, Some(6500));
    let without_humidity = SensorReport::new(51, 26.0_f32, 0x0000_00BB, 80, None);

    let mut blob = Vec::new();
    blob.extend(with_humidity.encode_frame());
    blob.extend(without_humidity.encode_frame());

    let mut dec = SensorReport::decoder();
    let mut got = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        for r in dec.iter() {
            got.push(r);
        }
    }

    assert_eq!(got.len(), 2);
    assert_eq!(got[0].humidity, Some(6500));
    assert_eq!(got[1].humidity, None);
    assert!(dec.buffered().is_empty());
}
