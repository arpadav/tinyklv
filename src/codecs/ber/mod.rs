// --------------------------------------------------
// external
// --------------------------------------------------
use crate::prelude::*;
use num_traits::{
    bounds::UpperBounded, AsPrimitive, FromPrimitive, ToBytes, ToPrimitive, Unsigned,
};
use winnow::token::{take, take_while};

// --------------------------------------------------
// local
// --------------------------------------------------
pub mod dec;
pub mod enc;

// --------------------------------------------------
// traits
// --------------------------------------------------
pub trait OfBerCommon:
    Copy
    + ToBytes
    + Unsigned
    + UpperBounded
    + PartialOrd
    + ToPrimitive
    + FromPrimitive
    + AsPrimitive<u128>
{
}
impl<T> OfBerCommon for T where
    T: Copy
        + ToBytes
        + Unsigned
        + UpperBounded
        + PartialOrd
        + ToPrimitive
        + FromPrimitive
        + AsPrimitive<u128>
{
}
pub trait OfBerLength: OfBerCommon {}
impl<T> OfBerLength for T where T: OfBerCommon {}
pub trait OfBerOid: OfBerCommon {}
impl<T> OfBerOid for T where T: OfBerCommon {}

#[derive(Debug, PartialEq)]
/// Enum representing Basic-Encoding-Rules (BER) Length Encoding.
///
/// Maximum precision: [`u128`]
///
/// * See: [https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf)
/// * See: [https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf](https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf) page 7
///
/// # Example
///
/// ```
/// use tinyklv::prelude::*;
/// use tinyklv::codecs::ber::BerLength;
///
/// assert_eq!(vec![128 + 3, 129, 182, 2], BerLength::new(8_500_738_u32).encode_value());
/// assert_eq!(BerLength::new(8_500_738_u32), BerLength::decode_value(&mut &vec![128 + 3, 129, 182, 2][..]).unwrap());
/// ```
pub enum BerLength<T: OfBerLength> {
    Short(u8),
    Long(T),
}
/// [`BerLength`] implementation
impl<T: OfBerLength> BerLength<T> {
    #[inline(always)]
    /// Checks if the input value can be represented as BER short form, which must
    /// be less than 128
    fn can_be_short(val: &T) -> bool {
        #![allow(
            clippy::expect_used,
            reason = "this should never panic, due to trait bounds"
        )]
        val < &T::from_u8(0x80).expect(
            "converting 128 -> u{8,16,32,64,128} should always be permissible, why did this panic?",
        )
    }

    /// Creates a new [BerLength] from a [`num_traits::Unsigned`]
    ///
    /// # Arguments
    ///
    /// * `len` - [`num_traits::Unsigned`]
    ///
    /// # Panics
    ///
    /// This should never panic, due to trait bounds
    pub fn new(len: T) -> Self {
        match Self::can_be_short(&len) {
            #[allow(
                clippy::expect_used,
                reason = "this should never panic, due to trait bounds"
            )]
            true => BerLength::Short(len.to_u8().expect("if unsigned int is less than 128, then it can always fit into u8, why did this panic?")),
            false => BerLength::Long(len),
        }
    }

    /// Encodes a length of [`BerLength`] into a [`Vec<u8>`]
    ///
    /// See [`BerLength`] implementation [`EncodeValue`]
    pub fn encode_value(len: T) -> Vec<u8> {
        Self::new(len).encode_value()
    }

    /// Returns the length as a [`u128`]
    pub fn as_u128(&self) -> u128 {
        match self {
            BerLength::Short(len) => *len as u128,
            BerLength::Long(len) => len.as_(),
        }
    }
}
/// [`BerLength`] implementation of [`EncodeValue`]
impl<T: OfBerLength> crate::EncodeValue<Vec<u8>> for BerLength<T> {
    /// Encode a [`BerLength`] into a [`Vec<u8>`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let value0 = BerLength::new(47_u64);
    /// let value1 = BerLength::new(201_u64);
    /// let value2 = BerLength::new(123891829038102_u64);
    ///
    /// assert_eq!(value0.encode_value(), vec![47]);
    /// assert_eq!(value1.encode_value(), vec![128 + 1, 201]);
    /// assert_eq!(value2.encode_value(), vec![128 + 6, 112, 173, 208, 117, 220, 22]);
    ///
    /// // Can also directly encode:
    /// let value0_encoded = BerLength::encode_value(47_u64);
    /// let value1_encoded = BerLength::encode_value(201_u64);
    ///
    /// assert_eq!(value0_encoded, vec![47]);
    /// assert_eq!(value1_encoded, vec![128 + 1, 201]);
    /// ```
    fn encode_value(&self) -> Vec<u8> {
        match self {
            BerLength::Short(len) => vec![*len],
            BerLength::Long(len) => {
                // --------------------------------------------------
                // Edge case: If the length fits within a single byte, use the Short form.
                // --------------------------------------------------
                // This should never happen: upon creation, length is checked to be < 128
                // --------------------------------------------------
                if Self::can_be_short(len) {
                    #[allow(
                        clippy::expect_used,
                        reason = "this should never panic, due to trait bounds"
                    )]
                    return vec![len.to_u8().expect("if unsigned int is less than 128, then it can always fit into u8, why did this panic?")];
                }
                // --------------------------------------------------
                // skip leading zeroes
                // --------------------------------------------------
                let mut encoded = len
                    .to_be_bytes()
                    .as_ref()
                    .iter()
                    .skip_while(|&&b| b == 0)
                    .copied()
                    .collect::<Vec<u8>>();
                // --------------------------------------------------
                // prefix byte with MSB set to 1, followed by the length
                // --------------------------------------------------
                let prefix = 0b1000_0000 | (encoded.len() as u8);
                // --------------------------------------------------
                // prepend the prefix byte and return
                // --------------------------------------------------
                let mut result = Vec::with_capacity(encoded.len() + 1);
                result.push(prefix);
                result.append(&mut encoded);
                result
            }
        }
    }
}
/// [`BerLength`] implementation of [`crate::traits::DecodeValue`]
impl<T: OfBerLength> crate::DecodeValue<&[u8]> for BerLength<T> {
    /// Decode a [`BerLength`] from a [`&[u8]`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let value0 = vec![47];
    /// let value1 = vec![128 + 1, 201];
    /// let value2 = vec![128 + 6, 112, 173, 208, 117, 220, 22];
    ///
    /// assert_eq!(BerLength::decode_value(&mut &value0[..]).unwrap(), BerLength::new(47_u64));
    /// assert_eq!(BerLength::decode_value(&mut &value1[..]).unwrap(), BerLength::new(201_u64));
    /// assert_eq!(BerLength::decode_value(&mut &value2[..]).unwrap(), BerLength::new(123891829038102_u64));
    /// ```
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // err if no bytes
        // --------------------------------------------------
        let first_byte = take_one(input)?;
        let first_byte = first_byte[0];
        // --------------------------------------------------
        // if MSB is not set, it's a short length (single byte)
        // --------------------------------------------------
        if first_byte & 0x80 == 0 {
            return Ok(BerLength::Short(first_byte));
        }
        // --------------------------------------------------
        // extract the number of bytes used for length encoding
        // --------------------------------------------------
        let num_bytes = (first_byte & 0x7F) as usize;
        // --------------------------------------------------
        // ensure there are enough bytes in the stream
        // --------------------------------------------------
        // since 1 was taken from input, this should be
        // `input.len() + 1 < num_bytes + 1`
        // but can be shortened
        // --------------------------------------------------
        if input.len() < num_bytes {
            return Err(winnow::error::ContextError::new()
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("BER-OID value"),
                )
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Expected(
                        winnow::error::StrContextValue::Description(
                            "enough bytes in stream for length encoding",
                        ),
                    ),
                ));
        }
        // --------------------------------------------------
        // decode the length from the specified number of bytes
        // --------------------------------------------------
        let output = match T::from_u128(parse_length_u128(input, num_bytes)?) {
            Some(value) => value,
            None => {
                return Err(winnow::error::ContextError::new()
                    .add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Label("BER-OID value"),
                    )
                    .add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Expected(
                            winnow::error::StrContextValue::Description("less than u128::MAX"),
                        ),
                    ));
            }
        };
        Ok(BerLength::Long(output))
    }
}

#[derive(Debug, PartialEq)]
/// Struct representing Basic Encoding Rules (BER) Object Identifier (OID) encoding.
///
/// Maximum precision: [`u128`]
///
/// * See: [https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf)
/// * See: [https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf](https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf) page 7
///
/// # Example
///
/// ```
/// use tinyklv::prelude::*;
/// use tinyklv::codecs::ber::BerOid;
///
/// assert_eq!(vec![129, 182, 2], BerOid::encode_value(23298_u64));
/// assert_eq!(23298_u64, BerOid::decode_value(&mut &vec![129, 182, 2][..]).unwrap().value);
/// ```
pub struct BerOid<T: OfBerOid> {
    pub value: T,
}
/// [`BerOid`] implementation
impl<T: OfBerOid> BerOid<T> {
    /// Creates a new [`BerOid`] from an unsigned integer
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Encodes a value of [`BerOid`] into a [`Vec<u8>`]
    pub fn encode_value(value: T) -> Vec<u8> {
        Self::new(value).encode_value()
    }
}
/// [`BerOid`] implementation of [`crate::traits::EncodeValue`]
impl<T: OfBerOid> crate::EncodeValue<Vec<u8>> for BerOid<T> {
    /// Encode a [`BerOid`] into a [`Vec<u8>`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// assert_eq!(vec![129, 182, 2], BerOid::encode_value(23298_u64));
    /// ```
    ///
    /// Please use [`crate::codecs::ber::enc::ber_oid`] instead for
    /// all parsing needs. This struct is meant to be used as a development
    /// tool for encoding values to BER format.
    fn encode_value(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut value = self.value.as_();
        let mut first_byte = true;
        while value > 0 {
            // --------------------------------------------------
            // extract 7 bits at a time
            // --------------------------------------------------
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            match first_byte {
                // --------------------------------------------------
                // LSB side of entire encoding has MSB set to 0
                // --------------------------------------------------
                true => {
                    first_byte = false;
                    output.push(byte);
                }
                // --------------------------------------------------
                // All remaining MSB-sided bytes have MSB set to 1
                // --------------------------------------------------
                false => output.push(byte | 0x80),
            }
        }
        output.reverse();
        output
    }
}
/// [`BerOid`] implementation of [`crate::traits::DecodeValue`]
impl<T: OfBerOid> crate::DecodeValue<&[u8]> for BerOid<T> {
    /// Decode a [`BerOid`] from a [`&[u8]`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// assert_eq!(23298_u64, BerOid::decode_value(&mut &vec![129, 182, 2][..]).unwrap().value);
    /// ```
    ///
    /// Please use [`crate::codecs::ber::dec::ber_oid`] instead for
    /// all parsing needs. This struct is meant to be used as a development
    /// tool for parsing BER encoded values.
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // BER-OID grammar: `(msb-set)* (msb-unset)`
        //   - zero or more continuation bytes with MSB = 1
        //   - exactly one terminator byte with MSB = 0
        // --------------------------------------------------
        let prefix: &[u8] = take_while(0.., msb_is_set).parse_next(input).map_err(
            |e: winnow::error::ContextError| {
                e.add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("BER-OID continuation bytes"),
                )
            },
        )?;
        let terminator =
            winnow::binary::be_u8(input).map_err(|e: winnow::error::ContextError| {
                e.add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label(
                        "BER-OID missing terminator byte (MSB unset) at end of input",
                    ),
                )
            })?;
        // --------------------------------------------------
        // accumulate 7 bits per byte, prefix first then terminator
        // --------------------------------------------------
        let output = prefix
            .iter()
            .copied()
            .chain(std::iter::once(terminator))
            .fold(0u128, |acc, b| (acc << 7) | (b & 0x7F) as u128);
        let output = match T::from_u128(output) {
            Some(value) => value,
            None => {
                return Err(winnow::error::ContextError::new()
                    .add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Label("BER-OID value"),
                    )
                    .add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Expected(
                            winnow::error::StrContextValue::Description("less than u128::MAX"),
                        ),
                    ));
            }
        };
        Ok(BerOid::new(output))
    }
}

#[inline(always)]
/// Parses out a single byte, returning it as a 1-element slice
fn take_one<'s>(input: &mut &'s [u8]) -> crate::Result<&'s [u8]> {
    take(1usize).parse_next(input)
}

#[inline(always)]
/// Checks if the MSB is set
fn msb_is_set(b: u8) -> bool {
    (b & 0x80) != 0
}

#[inline(always)]
/// Parses out a specified number of bytes and combines them into a [`u128`] value
fn parse_length_u128(input: &mut &[u8], num_bytes: usize) -> crate::Result<u128> {
    take(num_bytes)
        .map(|bytes: &[u8]| {
            bytes
                .iter()
                .fold(0u128, |acc, &byte| (acc << 8) | byte as u128)
        })
        .parse_next(input)
}
