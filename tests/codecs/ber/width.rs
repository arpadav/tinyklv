//! [`BerWidth`]: the integer types the BER codecs decode into, and the wires on each one's
//! boundaries
//!
//! A BER-OID run holds 7 value bits per byte, so a `bits`-wide type holds at most
//! `ceil(bits / 7)` bytes of it (its cap). A BER long-form length holds 8 per byte and is bounded
//! by its value alone, so a leading zero length byte is accepted. Every wire is built from the
//! width, and one assertion covers `u8` through `u128` and `usize`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::helpers::DecoderExt;
use tinyklv::codecs::ber::{BerLength, OfBerCommon};
use tinyklv::dec::ber as decber;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{fmt::Debug, iter};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The marker bit of a byte that continues a BER-OID run, and of a long-form BER length's first
/// byte
const MSB: u8 = 0x80;

/// An integer type the BER codecs decode into
pub(super) trait BerWidth: OfBerCommon + Debug {
    /// The type's largest value
    const MAX: Self;

    /// The type's width in bytes
    const BYTES: usize = size_of::<Self>();

    /// Bytes in the longest BER-OID run the type holds, terminator included
    const OID_CAP: usize = (Self::BYTES * 8).div_ceil(7);

    /// Value bits in the first group of an [`BerWidth::OID_CAP`]-byte run; every later group
    /// carries 7. Between 1 and 6 here, since no width is a multiple of 7
    const OID_TOP_BITS: usize = Self::BYTES * 8 - 7 * (Self::OID_CAP - 1);

    /// [`BerWidth::MAX`] as a BER-OID run: [`BerWidth::OID_CAP`] bytes, the first group full
    fn oid_max_run() -> Vec<u8> {
        let top = MSB | ((1 << Self::OID_TOP_BITS) - 1);
        iter::once(top)
            .chain(iter::repeat_n(0xFF, Self::OID_CAP - 2))
            .chain(iter::once(0x7F))
            .collect()
    }

    /// `MAX + 1` as a BER-OID run: [`BerWidth::OID_CAP`] bytes, the first group one past full
    fn oid_overflowing_run() -> Vec<u8> {
        let top = MSB | (1 << Self::OID_TOP_BITS);
        iter::once(top)
            .chain(iter::repeat_n(MSB, Self::OID_CAP - 2))
            .chain(iter::once(0x00))
            .collect()
    }

    /// [`BerWidth::OID_CAP`] bytes that all continue: the last stands where the terminator must
    fn oid_unterminated_run() -> Vec<u8> {
        vec![MSB | 0x01; Self::OID_CAP]
    }

    /// One byte short of [`BerWidth::OID_CAP`], all continuing: a terminator may still follow
    fn oid_pending_run() -> Vec<u8> {
        vec![MSB | 0x01; Self::OID_CAP - 1]
    }

    /// A long-form BER length: its count byte, then `lead` if any, then [`BerWidth::BYTES`]
    /// copies of `fill`
    ///
    /// # Arguments
    ///
    /// * `lead` - a length byte ahead of the type's width; `0x00` makes the form one byte longer
    ///   than the value needs
    /// * `fill` - each of the [`BerWidth::BYTES`] length bytes after `lead`
    fn length(lead: Option<u8>, fill: u8) -> Vec<u8> {
        let count = Self::BYTES + usize::from(lead.is_some());
        iter::once(MSB | count as u8)
            .chain(lead)
            .chain(iter::repeat_n(fill, Self::BYTES))
            .collect()
    }

    /// Asserts the BER-OID decoder's boundaries at this width
    ///
    /// The run of [`BerWidth::MAX`] reads; the run of `MAX + 1`, its first group one past full,
    /// overflows; a continuing byte where the terminator must be is rejected on the spot, without
    /// waiting for more bytes; a continuing run short of the cap runs out of bytes. Each
    /// rejection leaves the cursor as [`DecoderExt`] pins
    ///
    /// # Panics
    ///
    /// If any boundary does not hold
    #[track_caller]
    fn assert_oid_cap_boundary() {
        decber::ber_oid::<Self>.assert_reads(&Self::oid_max_run(), Self::MAX);
        decber::ber_oid::<Self>.assert_rewinds(&Self::oid_overflowing_run());
        decber::ber_oid::<Self>.assert_rewinds(&Self::oid_unterminated_run());
        decber::ber_oid::<Self>.assert_runs_out(&Self::oid_pending_run());
    }

    /// Asserts the [`BerLength`] decoder's boundaries at this width
    ///
    /// [`BerWidth::MAX`] in [`BerWidth::BYTES`] length bytes reads, and so does the same behind
    /// a zero length byte; `MAX + 1`, one length byte longer, overflows and rewinds
    ///
    /// # Panics
    ///
    /// If any boundary does not hold
    #[track_caller]
    fn assert_length_boundary() {
        let decode = |input: &mut &[u8]| BerLength::<Self>::decode_value(input);
        decode.assert_reads(&Self::length(None, 0xFF), BerLength::Long(Self::MAX));
        decode.assert_reads(&Self::length(Some(0x00), 0xFF), BerLength::Long(Self::MAX));
        decode.assert_rewinds(&Self::length(Some(0x01), 0x00));
    }
}

/// Implements [`BerWidth`] for each unsigned integer the BER codecs decode into
macro_rules! ber_width {
    ($($ty:ty),* $(,)?) => {
        $(
            /// The width's [`BerWidth::MAX`] is its own
            impl BerWidth for $ty {
                const MAX: Self = <$ty>::MAX;
            }
        )*
    };
}
ber_width!(u8, u16, u32, u64, u128, usize);
