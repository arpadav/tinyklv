//! The `rich` benchmark record: every field a common native Rust type, decoded straight into its
//! final form via tinyklv's `bench`-gated native `DecodeValue`/`EncodeValue` impls.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{GpsCoord, RngSample};
use tinyklv::{
    dec::binary as decb, dec::string as decs, enc::binary as encb, enc::string as encs, prelude::*,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use core::num::NonZeroU32;
use rand::{RngExt, rngs::SmallRng};
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

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
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    trait_fallback,
)]
pub(crate) struct Rich {
    #[klv(key = 0x01)]
    pub(crate) id: u32,

    #[klv(key = 0x02, size(exact = 20))]
    pub(crate) coord: GpsCoord,

    #[klv(key = 0x03, size(exact = 8))]
    pub(crate) timestamp: DateTime<Utc>,

    #[klv(key = 0x04, size(exact = 8))]
    pub(crate) elapsed: Duration,

    #[klv(key = 0x05, size(exact = 4))]
    pub(crate) addr: Ipv4Addr,

    #[klv(key = 0x06, size(exact = 16))]
    pub(crate) addr6: Ipv6Addr,

    #[klv(key = 0x07, size(exact = 4))]
    pub(crate) date: NaiveDate,

    #[klv(key = 0x08, size(exact = 4))]
    pub(crate) time: NaiveTime,

    #[klv(key = 0x09, size(exact = 4))]
    pub(crate) symbol: char,

    #[klv(key = 0x0A, size(exact = 4))]
    pub(crate) seq: NonZeroU32,

    #[klv(key = 0x0B, size(exact = 1))]
    pub(crate) flag: bool,

    #[klv(
        key = 0x0C,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(var, hint = 16),
    )]
    pub(crate) label: String,
}

/// [`Rich`] implementation of [`RngSample`]
impl RngSample for Rich {
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
        Rich {
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
