// ANCHOR: all
//! book 7.4 - tokio
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"SENSOR",
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
)]
/// One sensor reading
struct Sensor {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Frame counter
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
}

#[tokio::main]
async fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
    };

    #[rustfmt::skip]
    let wire = [
        0xDE, 0xAD, 0x00, 0x00, // junk
        // "SENSOR"
        0x53, 0x45, 0x4E, 0x53, 0x4F, 0x52,
        0x07,                   // length: 7 bytes
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    let mut encoded = Vec::new();
    sensor.encode_frame(&mut encoded);

    // a sender task: the bytes arrive 6 at a time
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4);
    let sender = tokio::spawn(async move {
        for chunk in wire.chunks(6) {
            tx.send(chunk.to_vec()).await.unwrap();
        }
    });

    // ANCHOR: recv
    let mut decoder = Sensor::decoder();
    let mut decoded = Vec::new();
    while let Some(chunk) = rx.recv().await {
        decoder.feed(&chunk);
        // every frame now whole; a cut one waits
        let ready: Vec<Sensor> = decoder.iter().collect();
        println!("recv {chunk:02X?} -> {ready:?}");
        decoded.extend(ready);
    }
    // ANCHOR_END: recv

    sender.await.unwrap();
    // the encoder writes no junk: its frame starts at the sentinel
    assert_eq!(encoded, wire[4..]);
    assert_eq!(decoded, [sensor]);
    assert_eq!(decoder.buffered(), []);
}
// ANCHOR_END: all
