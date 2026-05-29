//! Shared data model for the benchmark suite.
//!
//! These are the records every approach round-trips. They necessarily carry each
//! approach's `#[derive]` / attributes at the definition site (Rust attaches derives to
//! the type), so this file is "the common types plus their derives", not pure data; all
//! procedural per-approach code lives under [`super::approaches`]. The byte-packing of a
//! [`Reading`] run is shared by serde_klv, tlv_parser, and manual, so it lives here as
//! inherent methods rather than in any one approach.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::{dec::binary as decb, dec::string as decs, enc::binary as encb, enc::string as encs, prelude::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use core::num::NonZeroU32;
use rand::rngs::SmallRng;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Generates a random sample value for a record type, replacing standalone fixture fns
///
/// Implemented by every record type in the benchmark suite so that the criterion
/// driver and the CSV harness can each obtain varied instances. Each call advances
/// the provided RNG, producing different data on every invocation.
pub(crate) trait RngSample {
    /// Returns a random instance from the provided RNG
    ///
    /// The caller controls determinism by seeding the RNG before passing it in.
    fn rng_sample(rng: &mut SmallRng) -> Self;
}

/// Flat telemetry record: eight primitive fields. tinyklv and serde_klv both derive it.
#[derive(tinyklv::Klv, Serialize, Deserialize, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
#[serde(rename = "TELEMETRY0000000")]
pub(crate) struct Telemetry {
    #[klv(key = 0x01)]
    #[serde(rename = "1")]
    pub(crate) a: u8,

    #[klv(key = 0x02)]
    #[serde(rename = "2")]
    pub(crate) b: u16,

    #[klv(key = 0x03)]
    #[serde(rename = "3")]
    pub(crate) c: u32,

    #[klv(key = 0x04)]
    #[serde(rename = "4")]
    pub(crate) d: u64,

    #[klv(key = 0x05)]
    #[serde(rename = "5")]
    pub(crate) e: i16,

    #[klv(key = 0x06)]
    #[serde(rename = "6")]
    pub(crate) f: i32,

    #[klv(key = 0x07)]
    #[serde(rename = "7")]
    pub(crate) g: f32,

    #[klv(key = 0x08)]
    #[serde(rename = "8")]
    pub(crate) h: f64,
}

/// [`Telemetry`] implementation of [`RngSample`]
impl RngSample for Telemetry {
    fn rng_sample(rng: &mut SmallRng) -> Self {
        Telemetry {
            a: rng.random(),
            b: rng.random(),
            c: rng.random(),
            d: rng.random(),
            e: rng.random(),
            f: rng.random(),
            g: rng.random(),
            h: rng.random(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
/// A fixed 5-byte raw record: a `kind` tag plus a big-endian `f32`. Packed back-to-back
/// into the length-delimited `sensors` field; the byte-packing is shared by serde_klv,
/// tlv_parser, and the manual approach.
pub(crate) struct Reading {
    pub(crate) kind: u8,
    pub(crate) value: f32,
}

/// [`Reading`] implementation
impl Reading {
    /// Fixed wire width of one reading: `kind` (1 byte) + `value` (4 bytes big-endian `f32`)
    pub(crate) const WIDTH: usize = 5;

    /// Serialises a slice of readings into a packed byte run
    ///
    /// Each reading occupies exactly [`Reading::WIDTH`] bytes: the `kind` tag
    /// followed by the `value` as a big-endian `f32`
    ///
    /// # Arguments
    ///
    /// * `readings` - the slice of readings to pack; may be empty
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` of length `readings.len() * Reading::WIDTH`
    pub(crate) fn pack(readings: &[Reading]) -> Vec<u8> {
        // --------------------------------------------------
        // allocate output buffer
        // --------------------------------------------------
        let mut out = Vec::with_capacity(readings.len() * Reading::WIDTH);
        // --------------------------------------------------
        // pack each reading as kind + big-endian f32
        // --------------------------------------------------
        for r in readings {
            out.push(r.kind);
            out.extend_from_slice(&r.value.to_be_bytes());
        }
        out
    }

    /// Deserialises a packed byte run back into a `Vec<Reading>`
    ///
    /// Splits `bytes` into [`Reading::WIDTH`]-byte chunks and converts each one
    /// into a `Reading`. The `kind` is the first byte; `value` is the next four
    /// bytes interpreted as a big-endian `f32`
    ///
    /// # Arguments
    ///
    /// * `bytes` - the packed byte buffer to unpack
    ///
    /// # Returns
    ///
    /// `Some(Vec<Reading>)` when `bytes.len()` is an exact multiple of
    /// [`Reading::WIDTH`], or `None` if there is a trailing partial record
    pub(crate) fn unpack(bytes: &[u8]) -> Option<Vec<Reading>> {
        // --------------------------------------------------
        // split into fixed-width chunks and convert each to a reading
        // --------------------------------------------------
        let mut chunks = bytes.chunks_exact(Reading::WIDTH);
        let out = chunks
            .by_ref()
            .map(|c| Reading {
                kind: c[0],
                value: f32::from_be_bytes([c[1], c[2], c[3], c[4]]),
            })
            .collect();
        // --------------------------------------------------
        // reject trailing partial record
        // --------------------------------------------------
        chunks.remainder().is_empty().then_some(out)
    }
}

/// [`Reading`] implementation of [`tinyklv::DecodeValue`] for [`&[u8]`]
impl tinyklv::DecodeValue<&[u8]> for Reading {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        // --------------------------------------------------
        // read kind tag and value field
        // --------------------------------------------------
        let kind = decb::u8(input)?;
        let value = decb::be_f32(input)?;
        // --------------------------------------------------
        // construct reading
        // --------------------------------------------------
        Ok(Reading { kind, value })
    }
}

/// Nested coordinate sub-packet.
#[derive(tinyklv::Klv, Debug, PartialEq, Clone, Copy)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
pub(crate) struct GpsCoord {
    #[klv(key = 0x01)]
    pub(crate) lat: f64,

    #[klv(key = 0x02)]
    pub(crate) lon: f64,
}

/// Nested platform record: a sub-packet coordinate and a packed sensor run alongside
/// primitives. tinyklv derives it directly; serde_klv needs a hand-written impl.
#[derive(tinyklv::Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
    trait_fallback,
)]
pub(crate) struct Platform {
    #[klv(key = 0x01)]
    pub(crate) id: u32,

    #[klv(key = 0x02)]
    pub(crate) coord: GpsCoord,

    #[klv(key = 0x03)]
    pub(crate) vx: i16,

    #[klv(key = 0x04)]
    pub(crate) vy: i16,

    #[klv(key = 0x05)]
    pub(crate) vz: i16,

    #[klv(key = 0x06)]
    pub(crate) altitude: f64,

    #[klv(key = 0x07)]
    pub(crate) heading: f32,

    #[klv(key = 0x08)]
    pub(crate) mode: u8,

    #[klv(key = 0x09, enc = Reading::pack)]
    pub(crate) sensors: Vec<Reading>,
}

/// [`Platform`] implementation of [`RngSample`]
impl RngSample for Platform {
    fn rng_sample(rng: &mut SmallRng) -> Self {
        let n_sensors = (rng.random::<u8>() % 8) + 1;
        let sensors = (0..n_sensors)
            .map(|_| Reading {
                kind: rng.random(),
                value: rng.random(),
            })
            .collect();
        Platform {
            id: rng.random(),
            coord: GpsCoord {
                lat: rng.random(),
                lon: rng.random(),
            },
            vx: rng.random(),
            vy: rng.random(),
            vz: rng.random(),
            altitude: rng.random(),
            heading: rng.random(),
            mode: rng.random(),
            sensors,
        }
    }
}

/// Native-types record: every field is a common native Rust type, decoded straight into its
/// final form. tinyklv handles these via the `bench`-gated `DecodeValue`/`EncodeValue` impls in
/// `tinyklv::traits::native` - so the fields below carry NO per-field `dec`/`enc` (only `id` needs
/// the `u32` default codec, and `label` the built-in string codec). Every other approach must
/// convert from a raw scalar to the native type: the KLV approaches per-field, the protobuf crates
/// over a whole generated struct. `trait_fallback` routes the native fields to their trait impls.
#[derive(tinyklv::Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    trait_fallback,
)]
pub(crate) struct NativeNested {
    #[klv(key = 0x01)]
    pub(crate) id: u32,

    #[klv(key = 0x02)]
    pub(crate) coord: GpsCoord,

    #[klv(key = 0x03)]
    pub(crate) timestamp: DateTime<Utc>,

    #[klv(key = 0x04)]
    pub(crate) elapsed: Duration,

    #[klv(key = 0x05)]
    pub(crate) addr: Ipv4Addr,

    #[klv(key = 0x06)]
    pub(crate) addr6: Ipv6Addr,

    #[klv(key = 0x07)]
    pub(crate) date: NaiveDate,

    #[klv(key = 0x08)]
    pub(crate) time: NaiveTime,

    #[klv(key = 0x09)]
    pub(crate) symbol: char,

    #[klv(key = 0x0A)]
    pub(crate) seq: NonZeroU32,

    #[klv(key = 0x0B)]
    pub(crate) flag: bool,

    #[klv(key = 0x0C, varlen = true, dec = decs::to_string_utf8, enc = &encs::from_string_utf8)]
    pub(crate) label: String,
}

/// [`NativeNested`] implementation of [`RngSample`]
impl RngSample for NativeNested {
    fn rng_sample(rng: &mut SmallRng) -> Self {
        // --------------------------------------------------
        // bound the timestamp to a positive post-1970 range (< ~year 2033) so the protobuf WKT
        // SystemTime bridge (prost / rust_protobuf) stays in range and round-trips exactly
        // --------------------------------------------------
        let ts_nanos = i64::try_from(rng.random::<u64>() % 2_000_000_000_000_000_000)
            .expect("bounded below i64::MAX");
        // --------------------------------------------------
        // build a short ascii label (1..=16 bytes, well under the 1-byte KLV length limit)
        // --------------------------------------------------
        let label_len = usize::from(rng.random::<u8>() % 16 + 1);
        let label: String = (0..label_len)
            .map(|_| char::from(b'a' + rng.random::<u8>() % 26))
            .collect();
        NativeNested {
            id: rng.random(),
            coord: GpsCoord {
                lat: rng.random(),
                lon: rng.random(),
            },
            timestamp: DateTime::from_timestamp_nanos(ts_nanos),
            elapsed: Duration::from_nanos(rng.random()),
            addr: Ipv4Addr::from(rng.random::<u32>()),
            addr6: Ipv6Addr::from(rng.random::<u128>()),
            date: NaiveDate::from_num_days_from_ce_opt((rng.random::<u32>() % 700_000 + 1) as i32)
                .expect("days-from-ce in valid range"),
            time: NaiveTime::from_num_seconds_from_midnight_opt(rng.random::<u32>() % 86_400, 0)
                .expect("seconds-from-midnight in [0, 86400)"),
            symbol: rng.random(),
            seq: NonZeroU32::new(rng.random::<u32>() | 1).expect("nonzero by `| 1`"),
            flag: rng.random(),
            label,
        }
    }
}
