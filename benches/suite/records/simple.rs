//! The `simple` benchmark record: eight flat primitive fields.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::RngSample;
use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use rand::{RngExt, rngs::SmallRng};
use serde::{Deserialize, Serialize};

/// Flat telemetry record: eight primitive fields. tinyklv and serde_klv both derive it.
#[derive(tinyklv::Klv, Serialize, Deserialize, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
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
pub(crate) struct Simple {
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

/// [`Simple`] implementation of [`RngSample`]
impl RngSample for Simple {
    fn rng_sample(rng: &mut SmallRng) -> Self {
        Simple {
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
