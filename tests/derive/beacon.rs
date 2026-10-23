//! The all-optional [`Beacon`] the `Vec<T>` decode tests share
//!
//! A message whose fields are all `Option` decodes from zero bytes, so `Vec<T>::decode_value`
//! without its zero-progress guard would re-attempt it forever. Decoded as
//! [`AttemptBudget<Beacon>`](crate::attempt_budget::AttemptBudget), that loop panics instead of
//! hanging the suite. `decode_partial` needs no bound: a beacon streamed from zero bytes pauses
//! with `NeedMore`, which ends the `Vec<T>` loop
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// A beacon whose fields are all optional, so it decodes from zero bytes without error
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
pub(crate) struct Beacon {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    pub(crate) altitude: Option<u16>,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    pub(crate) heading: Option<u8>,
}
