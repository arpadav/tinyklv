#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/12-repeated-extraction.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq, Clone, Copy)]
#[klv(
    stream = &[u8],
    sentinel = b"SENS",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// A single sensor reading, sentinel-framed for batch extraction
struct SensorReading {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8
    )]
    /// Sensor identifier (0..=255)
    sensor_id: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_i32,
        enc = *encb::be_i32
    )]
    /// Reading value in fixed-point centi-units
    value_centiunit: i32,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32
    )]
    /// Milliseconds since capture started
    age_ms: u32,
}

fn main() {
    // produce five readings from one sampling window
    let original = vec![
        SensorReading { sensor_id: 1, value_centiunit:  2_350, age_ms:    0 },
        SensorReading { sensor_id: 2, value_centiunit:  2_400, age_ms:  100 },
        SensorReading { sensor_id: 1, value_centiunit:  2_355, age_ms:  200 },
        SensorReading { sensor_id: 3, value_centiunit: -1_000, age_ms:  350 },
        SensorReading { sensor_id: 2, value_centiunit:  2_398, age_ms:  500 },
    ];

    // concatenate the encoded frames into one buffer
    let batch: Vec<u8> = original
        .iter()
        .flat_map(|r| r.encode_frame())
        .collect();

    // peel them all off in one call - DrainFrames::drain_frames comes
    // from the prelude via the blanket impl for any T: DecodeFrame<S>
    let decoded = SensorReading::drain_frames(
        &mut batch.as_slice(),
    ).unwrap();

    assert_eq!(decoded, original);
}
