//! The `compound` benchmark record: a nested coordinate sub-packet and a packed sensor run
//! alongside primitives.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{GpsCoord, Reading, RngSample};
use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use rand::{RngExt, rngs::SmallRng};

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
pub(crate) struct Compound {
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

/// [`Compound`] implementation of [`RngSample`]
impl RngSample for Compound {
    fn rng_sample(rng: &mut SmallRng) -> Self {
        let n_sensors = (rng.random::<u8>() % 8) + 1;
        let sensors = (0..n_sensors)
            .map(|_| Reading {
                kind: rng.random(),
                value: rng.random(),
            })
            .collect();
        Compound {
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
