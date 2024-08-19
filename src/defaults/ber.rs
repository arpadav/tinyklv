use std::io::{self, Write, Read};

/// Trait for encoding to bytes.
trait Encode {
    fn encode(&self) -> Vec<u8>;
}

/// Trait for decoding from bytes.
trait Decode: Sized {
    fn decode(bytes: &[u8]) -> io::Result<Self>;
}

/// Enum representing BER Length Encoding.
enum BERLength {
    Short(u8),
    Long(Vec<u8>),
}

impl Encode for BERLength {
    fn encode(&self) -> Vec<u8> {
        match self {
            BERLength::Short(length) => vec![*length],
            BERLength::Long(length_bytes) => {
                let mut result = vec![0x80 | length_bytes.len() as u8];
                result.extend(length_bytes);
                result
            }
        }
    }
}

impl Decode for BERLength {
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No bytes provided"));
        }

        if bytes[0] <= 127 {
            Ok(BERLength::Short(bytes[0]))
        } else {
            let num_bytes = (bytes[0] & 0x7F) as usize;
            if bytes.len() < num_bytes + 1 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid BER length encoding"));
            }
            let length_bytes = bytes[1..=num_bytes].to_vec();
            Ok(BERLength::Long(length_bytes))
        }
    }
}

/// Enum representing BER-OID encoding for tags.
enum BEROIDTag {
    SingleByte(u8),
    MultiByte(Vec<u8>),
}

impl Encode for BEROIDTag {
    fn encode(&self) -> Vec<u8> {
        match self {
            BEROIDTag::SingleByte(tag) => vec![*tag],
            BEROIDTag::MultiByte(tag_bytes) => tag_bytes.clone(),
        }
    }
}

impl Decode for BEROIDTag {
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No bytes provided"));
        }

        let mut tag = 0u32;
        for &byte in bytes {
            tag = (tag << 7) | (byte & 0x7F) as u32;
            if byte & 0x80 == 0 {
                return if tag < 128 {
                    Ok(BEROIDTag::SingleByte(tag as u8))
                } else {
                    Ok(BEROIDTag::MultiByte(bytes.to_vec()))
                };
            }
        }

        Err(io::Error::new(io::ErrorKind::InvalidInput, "Incomplete BER-OID tag"))
    }
}

/// Enum representing a BER Key-Length-Value structure.
enum BERKLV {
    Key(BEROIDTag),
    Length(BERLength),
    Value(Vec<u8>),
}

impl Encode for BERKLV {
    fn encode(&self) -> Vec<u8> {
        match self {
            BERKLV::Key(key) => key.encode(),
            BERKLV::Length(length) => length.encode(),
            BERKLV::Value(value) => value.clone(),
        }
    }
}

impl Decode for BERKLV {
    fn decode(bytes: &[u8], klv_type: KLVType) -> io::Result<Self> {
        match klv_type {
            KLVType::Key => Ok(BERKLV::Key(BEROIDTag::decode(bytes)?)),
            KLVType::Length => Ok(BERKLV::Length(BERLength::decode(bytes)?)),
            KLVType::Value => Ok(BERKLV::Value(bytes.to_vec())),
        }
    }
}

/// Enum to specify the type of Key-Length-Value decoding.
enum KLVType {
    Key,
    Length,
    Value,
}

/// Struct for encoding/decoding a BER object.
struct BERObject {
    klv: Vec<BERKLV>,
}

impl BERObject {
    fn new(klv: Vec<BERKLV>) -> Self {
        BERObject { klv }
    }

    /// Encode the entire BERObject.
    fn encode(&self) -> Vec<u8> {
        self.klv.iter().flat_map(|item| item.encode()).collect()
    }

    /// Decode the entire BERObject from bytes.
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let mut klv = Vec::new();
        let mut idx = 0;

        // Assuming a specific structure: Key, Length, and then Value.
        while idx < bytes.len() {
            // Decode Key
            let key = BERKLV::decode(&bytes[idx..], KLVType::Key)?;
            idx += match &key {
                BERKLV::Key(BEROIDTag::SingleByte(_)) => 1,
                BERKLV::Key(BEROIDTag::MultiByte(bytes)) => bytes.len(),
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid key")),
            };

            // Decode Length
            let length = BERKLV::decode(&bytes[idx..], KLVType::Length)?;
            idx += match &length {
                BERKLV::Length(BERLength::Short(_)) => 1,
                BERKLV::Length(BERLength::Long(bytes)) => bytes.len() + 1,
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid length")),
            };

            // Decode Value
            let value = match &length {
                BERKLV::Length(BERLength::Short(len)) => BERKLV::decode(&bytes[idx..idx + (*len as usize)], KLVType::Value)?,
                BERKLV::Length(BERLength::Long(len_bytes)) => {
                    let mut len = 0usize;
                    for byte in len_bytes {
                        len = (len << 8) | (*byte as usize);
                    }
                    BERKLV::decode(&bytes[idx..idx + len], KLVType::Value)?
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid length")),
            };
            idx += match &value {
                BERKLV::Value(v) => v.len(),
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid value")),
            };

            klv.push(key);
            klv.push(length);
            klv.push(value);
        }

        Ok(BERObject::new(klv))
    }
}

fn main() {
    // Encode a BER object with a Key, Length, and Value.
    let key = BERKLV::Key(ber_oid_encode_tag(23298));
    let length = BERKLV::Length(ber_length_encode(5));
    let value = BERKLV::Value(vec![1, 2, 3, 4, 5]);
    
    let ber_object = BERObject::new(vec![key, length, value]);
    let encoded_bytes = ber_object.encode();
    println!("Encoded BER Object: {:?}", encoded_bytes);
    
    // Decode the BER object from bytes.
    let decoded_object = BERObject::decode(&encoded_bytes).unwrap();
    println!("Decoded BER Object: {:?}", decoded_object.encode());
}
