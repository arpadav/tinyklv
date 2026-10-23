//! The BER codecs checked against [`X690`], the standard's rules written out: every wire of up
//! to two octets, random longer wires, random values, and every bit boundary of every width
//!
//! Each decoder must agree with the model on the value, the octets it reads, and where it leaves
//! the cursor when it fails. Each encoder must write the model's octets, which are the fewest
//! X.690 allows, and read back to the value it was given
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::model::{BIT_8, BITS_7_TO_1, GROUP_BITS, Outcome, X690};
use tinyklv::codecs::ber::{BerLength, BerOid, OfBerCommon};
use tinyklv::dec::ber as decber;
use tinyklv::enc::ber as encber;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::bounds::UpperBounded;
use proptest::{collection::vec, prelude::*};
use std::{fmt::Debug, iter};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The most octets a random wire carries after its prefix: past the longest BER-OID run (19
/// octets, at `u128`) and the longest long form a type holds (17), so a wire ends inside, at or
/// past either
const MAX_OCTETS: usize = 2 * size_of::<u128>();

/// The most zero octets a random long form carries ahead of its random ones: one more than the
/// widest type, so some forms carry more length octets than any type holds
const MAX_LEAD_ZEROS: usize = size_of::<u128>() + 1;

/// The chance a random BER-OID octet continues its run, high enough that some runs reach every
/// width's cap
const CONTINUES: f64 = 0.85;

/// The most `0x80` octets a random BER-OID run is padded with: one more than `u8`'s cap, so
/// padding alone can overflow it
const MAX_PADDING: usize = u8::BITS.div_ceil(GROUP_BITS) as usize + 1;

/// The random cases each property runs, eight times proptest's default
const CASES: u32 = 8 * 256;

/// A target type of the BER codecs, checked against [`X690`] at its width
trait Conforms: OfBerCommon + Debug {
    /// The type's width in bits
    const BITS: u32;

    /// Asserts each decoder into this type reads `wire` as [`X690`] does
    ///
    /// # Arguments
    ///
    /// * `wire` - all the decoders see
    ///
    /// # Errors
    ///
    /// If a decoder's [`Outcome`] differs from the model's
    fn assert_decodes(wire: &[u8]) -> Result<(), TestCaseError> {
        let length = Outcome::of(wire, |input| {
            BerLength::<Self>::decode_value(input).map(|length| length.value())
        });
        prop_assert_eq!(
            length,
            X690::length(wire, Self::BITS),
            "length {:02X?}",
            wire
        );
        let expected = X690::oid(wire, Self::BITS);
        let oid = Outcome::of(wire, decber::ber_oid::<Self>);
        prop_assert_eq!(&oid, &expected, "ber_oid {:02X?}", wire);
        let oid = Outcome::of(wire, |input| {
            BerOid::<Self>::decode_value(input).map(|oid| oid.value())
        });
        prop_assert_eq!(oid, expected, "BerOid {:02X?}", wire);
        Ok(())
    }

    /// Asserts each encoder writes `value` as [`X690`] does, and that the decoders read it back
    ///
    /// # Arguments
    ///
    /// * `value` - the value to encode
    ///
    /// # Errors
    ///
    /// If an encoder writes other octets than the model's, or a decoder reads them back to
    /// anything but `value`
    fn assert_encodes(value: Self) -> Result<(), TestCaseError> {
        let wide: u128 = value.as_();
        let mut length = Vec::new();
        encber::ber_length(value, &mut length);
        prop_assert_eq!(&length, &X690::encode_length(wide), "length {:?}", value);
        let read = Outcome::of(&length, |input| {
            BerLength::<Self>::decode_value(input).map(|length| length.value())
        });
        prop_assert_eq!(
            read,
            Outcome::Reads {
                value: wide,
                consumed: length.len()
            }
        );
        let mut oid = Vec::new();
        encber::ber_oid(value, &mut oid);
        prop_assert_eq!(&oid, &X690::encode_oid(wide), "oid {:?}", value);
        let read = Outcome::of(&oid, decber::ber_oid::<Self>);
        prop_assert_eq!(
            read,
            Outcome::Reads {
                value: wide,
                consumed: oid.len()
            }
        );
        Ok(())
    }

    /// Asserts each value one below, at and one above every power of two the type holds encodes
    /// as [`X690`] does and reads back: every octet count of both forms, `0`, `127`, `128`,
    /// `255`, `256` and the type's maximum among them
    ///
    /// # Errors
    ///
    /// If any value fails [`Conforms::assert_encodes`]
    fn assert_bit_boundaries() -> Result<(), TestCaseError> {
        let max: u128 = <Self as UpperBounded>::max_value().as_();
        let powers = (0..Self::BITS).map(|bit| 1_u128 << bit);
        let around = powers.flat_map(|power| [power - 1, power, power + 1]);
        for value in around.chain(iter::once(max)).filter(|&value| value <= max) {
            let Some(value) = Self::from_u128(value) else {
                return Err(TestCaseError::fail(format!("{value} does not fit")));
            };
            Self::assert_encodes(value)?;
        }
        Ok(())
    }
}

/// Implements [`Conforms`] for each unsigned integer the BER codecs read and write
macro_rules! conforms {
    ($($ty:ty),* $(,)?) => {
        $(
            /// The width's [`Conforms::BITS`] is its own
            impl Conforms for $ty {
                const BITS: u32 = <$ty>::BITS;
            }
        )*
    };
}
conforms!(u8, u16, u32, u64, u128, usize);

/// Runs one [`Conforms`] assertion at every width
macro_rules! every_width {
    ($assert:ident($($arg:expr),*)) => {
        <u8 as Conforms>::$assert($($arg),*)?;
        <u16 as Conforms>::$assert($($arg),*)?;
        <u32 as Conforms>::$assert($($arg),*)?;
        <u64 as Conforms>::$assert($($arg),*)?;
        <u128 as Conforms>::$assert($($arg),*)?;
        <usize as Conforms>::$assert($($arg),*)?;
    };
}

/// Wires of BER-length shape: a long-form initial octet with any count, then up to
/// [`MAX_LEAD_ZEROS`] zero octets and [`MAX_OCTETS`] random ones, so a value runs out, fits, or
/// overflows
fn length_wires() -> impl Strategy<Value = Vec<u8>> {
    let shape = (
        0..=BITS_7_TO_1,
        0..=MAX_LEAD_ZEROS,
        vec(any::<u8>(), 0..=MAX_OCTETS),
    );
    shape.prop_map(|(count, zeros, octets)| {
        iter::once(BIT_8 | count)
            .chain(iter::repeat_n(0, zeros))
            .chain(octets)
            .collect()
    })
}

/// Wires of BER-OID shape: up to [`MAX_OCTETS`] octets, each continuing the run with chance
/// [`CONTINUES`], behind up to [`MAX_PADDING`] `0x80` octets of leading padding
fn oid_wires() -> impl Strategy<Value = Vec<u8>> {
    let octet = (prop::bool::weighted(CONTINUES), 0..=BITS_7_TO_1)
        .prop_map(|(continues, group)| if continues { BIT_8 | group } else { group });
    (0..=MAX_PADDING, vec(octet, 0..=MAX_OCTETS))
        .prop_map(|(padding, octets)| iter::repeat_n(BIT_8, padding).chain(octets).collect())
}

#[test]
/// Tests every decoder against the model on every wire of up to two octets, so every initial
/// octet, every one-octet value, every cut long form and run, and every second octet after each
fn every_wire_up_to_two_octets_decodes_as_x690() -> Result<(), TestCaseError> {
    let short = (0..=u8::MAX).map(|octet| vec![octet]);
    let pairs = (0..=u16::MAX).map(|pair| pair.to_be_bytes().to_vec());
    for wire in iter::once(Vec::new()).chain(short).chain(pairs) {
        every_width!(assert_decodes(&wire));
        let length = Outcome::of(&wire, decber::ber_length);
        prop_assert_eq!(
            length,
            X690::length(&wire, usize::BITS),
            "ber_length {:02X?}",
            wire
        );
    }
    Ok(())
}

#[test]
/// Tests every width's encoders against the model, and their round trip, at every bit boundary
fn every_bit_boundary_encodes_as_x690() -> Result<(), TestCaseError> {
    every_width!(assert_bit_boundaries());
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    /// Tests every decoder against the model on random wires: any octets, long forms, and runs
    fn random_wires_decode_as_x690(
        wire in prop_oneof![vec(any::<u8>(), 0..=MAX_OCTETS), length_wires(), oid_wires()],
    ) {
        every_width!(assert_decodes(&wire));
        let length = Outcome::of(&wire, decber::ber_length);
        prop_assert_eq!(length, X690::length(&wire, usize::BITS), "ber_length {:02X?}", wire);
    }

    #[test]
    /// Tests every width's encoders against the model, and their round trip, on random values
    fn random_values_encode_as_x690(bits in any::<u128>(), shift in 0..u128::BITS) {
        let value = bits >> shift;
        <u8 as Conforms>::assert_encodes(value as u8)?;
        <u16 as Conforms>::assert_encodes(value as u16)?;
        <u32 as Conforms>::assert_encodes(value as u32)?;
        <u64 as Conforms>::assert_encodes(value as u64)?;
        <u128 as Conforms>::assert_encodes(value)?;
        <usize as Conforms>::assert_encodes(value as usize)?;
    }
}
