// --------------------------------------------------
// external
// --------------------------------------------------
use std::{io, num::NonZero};
use num_traits::{
    ToBytes,
    Unsigned,
    AsPrimitive,
    ToPrimitive,
    FromPrimitive,
};
use winnow::error::{
    Needed,
    ErrMode,
};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

#[derive(Debug, PartialEq)]
/// Enum representing BER Length Encoding.
/// 
/// Maximum precision: [u128]
/// 
/// See: https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf 
pub enum BerLength<T>
where 
    T: OfBerLength
{
    Short(u8),
    Long(T),
}
pub trait OfBerLength: ToBytes + Unsigned + PartialOrd + ToPrimitive + FromPrimitive {}
impl<T> OfBerLength for T where T: ToBytes + Unsigned + PartialOrd + ToPrimitive + FromPrimitive {}

/// [BerLength] implementation
impl<T: OfBerLength> BerLength<T> {
    /// Creates a new [BerLength] from a [num_traits::Unsigned]
    /// 
    /// # Arguments
    /// 
    /// * `len` - [num_traits::Unsigned]
    /// 
    /// # Panics
    /// 
    /// This should never panic, due to trait bounds
    pub fn new(len: T) -> Self {
        match len < T::from_u8(128).unwrap() {
            true => BerLength::Short(len.to_u8().unwrap()),
            false => BerLength::Long(len),
        }
    }

    /// Encodes a length of [BerLength] into a [Vec<u8>]
    /// 
    /// See [BerLength] implementation [Encode] [Self::encode]
    pub fn encode(len: T) -> Vec<u8> {
        Self::new(len).encode()
    }
}

/// [BerLength] implementation of [Encode]
impl<T: OfBerLength> Encode for BerLength<T> {
    /// Encode a [BerLength] into a [Vec<u8>]
    /// 
    /// # Example
    /// 
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::defaults::ber::BerLength;
    /// 
    /// let value0 = BerLength::new(47_u64);
    /// let value1 = BerLength::new(201_u64);
    /// let value2 = BerLength::new(123891829038102_u64);
    /// 
    /// assert_eq!(value0.encode(), vec![47]);
    /// assert_eq!(value1.encode(), vec![128 + 1, 201]);
    /// assert_eq!(value2.encode(), vec![128 + 6, 112, 173, 208, 117, 220, 22]);
    /// 
    /// // Can also directly encode:
    /// let value0_encoded = BerLength::encode(47_u64);
    /// let value1_encoded = BerLength::encode(201_u64);
    /// 
    /// assert_eq!(value0_encoded, vec![47]);
    /// assert_eq!(value1_encoded, vec![128 + 1, 201]);
    /// ```
    fn encode(&self) -> Vec<u8> {
        match self {
            BerLength::Short(len) => vec![*len],
            BerLength::Long(len) => {
                // --------------------------------------------------
                // Edge case: If the length fits within a single byte, use the Short form.
                // --------------------------------------------------
                // This should never happen: upon creation, length is checked to be < 128
                // --------------------------------------------------
                if len < &T::from_u8(128).unwrap() { return vec![len.to_u8().unwrap()]; }
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

/// [BerLength] implementation of [FixedDecode]
impl<T: OfBerLength> FixedDecode for BerLength<T> {
    type Error = io::Error;
    /// Decode a [BerLength] from a [Vec<u8>]
    /// 
    /// # Example
    /// 
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::defaults::ber::BerLength;
    /// 
    /// assert_eq!(BerLength::fixed_decode(&[47]).unwrap(), BerLength::<u64>::Short(47_u8));
    /// assert_eq!(BerLength::fixed_decode(&[128 + 1, 201]).unwrap(), BerLength::Long(201_u64));
    /// assert_eq!(BerLength::fixed_decode(&[128 + 6, 112, 173, 208, 117, 220, 22]).unwrap(), BerLength::Long(123891829038102_u64));
    /// ```
    fn fixed_decode(input: &[u8]) -> Result<Self, Self::Error> {
        // --------------------------------------------------
        // err if no bytes
        // --------------------------------------------------
        if input.is_empty() {
            return Result::Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "No bytes provided"
            ));
        }
        let first_byte = input[0];
        // --------------------------------------------------
        // if MSB is not set, it's a short length (single byte)
        // --------------------------------------------------
        if first_byte & 0x80 == 0 { return Ok(BerLength::Short(first_byte)); }
        // --------------------------------------------------
        // extract the number of bytes used for length encoding
        // --------------------------------------------------
        let num_bytes = (first_byte & 0x7F) as usize;
        if input.len() < num_bytes + 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Insufficient bytes for BER length decoding",
            ));
        }
        // --------------------------------------------------
        // decode the length from the specified number of bytes
        // --------------------------------------------------
        let mut output = 0u128;
        for &byte in &input[1..=num_bytes] {
            output = (output << 8) | byte as u128;
        }
        Ok(BerLength::Long(T::from_u128(output).unwrap()))
    }
}

/// [BerLength] implementation of [StreamDecode]
impl<T: OfBerLength> StreamDecode for BerLength<T> {
    fn decode(input: &mut crate::Stream) -> winnow::PResult<Self> {
        // --------------------------------------------------
        // err if no bytes
        // --------------------------------------------------
        let first_byte = parsers::take_one(input)?;
        let first_byte = first_byte[0];
        // --------------------------------------------------
        // if MSB is not set, it's a short length (single byte)
        // --------------------------------------------------
        if first_byte & 0x80 == 0 { return Ok(BerLength::Short(first_byte)); }
        // --------------------------------------------------
        // extract the number of bytes used for length encoding
        // --------------------------------------------------
        let num_bytes = (first_byte & 0x7F) as usize;
        // --------------------------------------------------
        // ensure there are enough bytes in the stream
        // --------------------------------------------------
        if input.len() < num_bytes + 1 { return Err(ErrMode::Incomplete(Needed::Size(NonZero::new(num_bytes + 1).unwrap()))); }
        // --------------------------------------------------
        // decode the length from the specified number of bytes
        // --------------------------------------------------
        let output = parsers::parse_length(input, num_bytes)?;
        Ok(BerLength::Long(T::from_u128(output).unwrap()))
    }
}

#[derive(Debug, PartialEq)]
/// Struct representing BER-OID encoding for tags.
/// 
/// Maximum precision: [u64]
/// 
/// See: https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf
pub struct BerOid<T>
where 
    T: OfBerOid
{
    value: T,
}
pub trait OfBerOid: ToBytes + Unsigned + PartialOrd + AsPrimitive<u64> + ToPrimitive + FromPrimitive {}
impl<T> OfBerOid for T where T: ToBytes + Unsigned + PartialOrd + AsPrimitive<u64> + ToPrimitive + FromPrimitive {}

impl<T: OfBerOid> BerOid<T> {
    /// Creates a new [BerOid] from an unsigned integer
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Encodes a value of [BerOid] into a [Vec<u8>]
    pub fn encode(value: T) -> Vec<u8> {
        Self::new(value).encode()
    }
}

/// [BerOid] implementation of [Encode]
impl<T: OfBerOid> Encode for BerOid<T> {
    /// Encode a [BerOid] into a [Vec<u8>]
    /// 
    /// # Example
    /// 
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::defaults::ber::BerOid;
    /// 
    /// assert_eq!(vec![129, 182, 2], BerOid::encode(23298 as u64));
    /// ```
    fn encode(&self) -> Vec<u8> {
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
                },
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

/// [BerOid] implementation of [FixedDecode]
impl<T: OfBerOid> FixedDecode for BerOid<T> {
    type Error = io::Error;
    fn fixed_decode(input: &[u8]) -> io::Result<Self> {
        let mut value = 0u64;
        for &byte in input.iter() {
            // --------------------------------------------------
            // extract the 7 bits from the byte (ignoring the MSB)
            // and insert to correct position
            // --------------------------------------------------
            value = (value << 7) | (byte & 0x7F) as u64;
            // --------------------------------------------------
            // if MSB is 0, return
            // --------------------------------------------------
            if byte & 0x80 == 0 { return Ok(BerOid::new(T::from_u64(value).unwrap())); }
        }
        Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid BER OID"))
    }
}

/// [BerOid] implementation of [StreamDecode]
impl<T: OfBerOid> StreamDecode for BerOid<T> {
    fn decode(input: &mut crate::Stream) -> winnow::PResult<Self> {
        // --------------------------------------------------
        // take while MSB = 1, then take last byte and exit
        // if fails, return error
        // --------------------------------------------------
        let output = parsers::take_while_msb_set(input)?
            .iter()
            .chain(parsers::take_one(input)?)
            // --------------------------------------------------
            // extract the 7 bits from the byte (ignoring the MSB)
            // and insert to correct position
            // --------------------------------------------------
            .fold(0u64, |acc, &b| (acc << 7) | (b & 0x7F) as u64);
        Ok(BerOid::new(T::from_u64(output).unwrap()))
    }
}

pub mod parsers {
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::token::{
        take,
        take_while,
    };
    use winnow::prelude::*;

    #[inline]
    /// Parses out all bytes while MSB is set to 1
    pub fn take_while_msb_set<'s>(input: &mut crate::Stream<'s>) -> winnow::PResult<crate::Stream<'s>> {
        take_while(1.., msb_is_set).parse_next(input)
    }

    #[inline]
    /// Parses out a single byte. MSB is **assumed** set to 0, since
    /// this function is only called after [BerOid::take_while_msb_set]
    pub fn take_one<'s>(input: &mut crate::Stream<'s>) -> winnow::PResult<crate::Stream<'s>> {
        take(1usize).parse_next(input)
    }

    #[inline]
    /// Checks if the MSB is set
    pub fn msb_is_set(b: u8) -> bool {
        (b & 0x80) != 0
    }

    /// Parses out a specified number of bytes and combines them into a `u128` value
    pub fn parse_length<'s>(input: &mut crate::Stream<'s>, num_bytes: usize) -> winnow::PResult<u128> {
        take(num_bytes)
            .map(|bytes: &[u8]| bytes.iter().fold(0u128, |acc, &byte| (acc << 8) | byte as u128))
            .parse_next(input)
    }
}

// /// Enum representing a BER Key-Length-Value structure.
// struct BerKlv<K, L>
// where 
//     K: OfBerOid,
//     L: OfBerLength
// {
//     key: K,
//     len: std::marker::PhantomData<L>,
//     val: Vec<u8>,
// }

// impl<K: OfBerOid, L: OfBerLength> BerKlv<K, L> {
//     fn new(key: K, val: Vec<u8>) -> Self {
//         Self { key, len: std::marker::PhantomData, val }
//     }
// }

// impl<K: OfBerOid, L: OfBerLength> Encode for BerKlv<K, L> {
//     fn encode(&self) -> Vec<u8> {
//         BerOid::encode(self.key).into_iter()
//             .chain(BerLength::encode(self.val.len()).into_iter())
//             .chain(self.val.clone().into_iter())
//             .collect()
//     }
// }

// impl Decode for BERKLV {
//     fn decode(bytes: &[u8], klv_type: KLVType) -> io::Result<Self> {
//         match klv_type {
//             KLVType::Key => Ok(BERKLV::Key(BEROIDTag::decode(bytes)?)),
//             KLVType::Length => Ok(BERKLV::Length(BerLength::decode(bytes)?)),
//             KLVType::Value => Ok(BERKLV::Value(bytes.to_vec())),
//         }
//     }
// }

// /// Enum to specify the type of Key-Length-Value decoding.
// enum KLVType {
//     Key,
//     Length,
//     Value,
// }

// /// Struct for encoding/decoding a BER object.
// struct BERObject {
//     klv: Vec<BERKLV>,
// }

// impl BERObject {
//     fn new(klv: Vec<BERKLV>) -> Self {
//         BERObject { klv }
//     }

//     /// Encode the entire BERObject.
//     fn encode(&self) -> Vec<u8> {
//         self.klv.iter().flat_map(|item| item.encode()).collect()
//     }

//     /// Decode the entire BERObject from bytes.
//     fn decode(bytes: &[u8]) -> io::Result<Self> {
//         let mut klv = Vec::new();
//         let mut idx = 0;

//         // Assuming a specific structure: Key, Length, and then Value.
//         while idx < bytes.len() {
//             // Decode Key
//             let key = BERKLV::decode(&bytes[idx..], KLVType::Key)?;
//             idx += match &key {
//                 BERKLV::Key(BEROIDTag::SingleByte(_)) => 1,
//                 BERKLV::Key(BEROIDTag::MultiByte(bytes)) => bytes.len(),
//                 _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid key")),
//             };

//             // Decode Length
//             let length = BERKLV::decode(&bytes[idx..], KLVType::Length)?;
//             idx += match &length {
//                 BERKLV::Length(BerLength::Short(_)) => 1,
//                 BERKLV::Length(BerLength::Long(bytes)) => bytes.len() + 1,
//                 _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid length")),
//             };

//             // Decode Value
//             let value = match &length {
//                 BERKLV::Length(BerLength::Short(len)) => BERKLV::decode(&bytes[idx..idx + (*len as usize)], KLVType::Value)?,
//                 BERKLV::Length(BerLength::Long(len_bytes)) => {
//                     let mut len = 0usize;
//                     for byte in len_bytes {
//                         len = (len << 8) | (*byte as usize);
//                     }
//                     BERKLV::decode(&bytes[idx..idx + len], KLVType::Value)?
//                 }
//                 _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid length")),
//             };
//             idx += match &value {
//                 BERKLV::Value(v) => v.len(),
//                 _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid value")),
//             };

//             klv.push(key);
//             klv.push(length);
//             klv.push(value);
//         }

//         Ok(BERObject::new(klv))
//     }
// }

#[test]
fn main() {
    // let value0 = BerLength::new(47 as u64);
    // let value1 = BerLength::new(201 as u64);
    let value2 = BerLength::new(123891829038102 as u64);
    
    // assert_eq!(value0.encode(), vec![47]);
    println!("{:?}", value2.encode());
    // println!("{:?}", value0.encode()); // Should return [201]
    // println!("{:?}", value1.encode());  // Encoded long form of the number

    // let value3 = BerOid::new(23298 as u64);
    let mut value4 = BerOid::encode(23298 as u64);
    let value5 = BerOid::<u64>::decode(&mut value4.as_slice()).unwrap();
    println!("{:?}", value4);
    println!("{:?}", value5);
    // let value5 = value5.unwrap();

    // [129, 182, 2]
    // println!("{:?}", value4);

    // let encoded = value3.encode();
    // println!("{:?}", value3.encode());
}
