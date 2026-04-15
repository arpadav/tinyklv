mod dummy_trait;
use dummy_trait::DummyKlvTrait;

use tinyklv_impl::Klv;
use winnow::Parser;

struct InnerValue {}

fn ex01_encoder(_: &InnerValue) -> Vec<u8> {
    return vec![0x65, 0x66, 0x67, 0x68];
}

fn ex02_encoder(_: &InnerValue) -> Vec<u8> {
    return String::from("Y2K").into_bytes();
}

// impl EncodeValue<Vec<u8>> for InnerValue {
//     fn encode_value(&self) -> Vec<u8> {
//         return String::from("KLV").to_lowercase().into_bytes();
//     }
// }

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"\x00",
    key(enc = "enc_bar", dec = nested::layer::another::u8_dec),
    len(enc = enc_baz, dec = "nested::layer::another::u8_dec"),
    // anya(enc = enc_baz, dec = "nested::layer::another::u8_dec"),
    default(typ = u8, enc = u8, dec = dec_qux),
    default(typ = u16, enc = another, dec = something),
    allow_unimplemented_decode,
    // sometin,
    // default(typ = u84, enc = "enc_bar", dec = "dec_qux"),
)]
struct MyStruct {
    // a: u32,

    #[klv(key = 0x07, enc = "ex01_encoder")]
    example_one: InnerValue,

    #[klv(key = 0x0A)]
    #[klv(enc = ex02_encoder, var = false)]
    example_two: InnerValue,

    #[klv(key = 0x8A, enc = "InnerValue::encode_value")]
    example_three: InnerValue,
}

fn dec_foo(input: &mut &[u8]) -> winnow::PResult<u8> {
    winnow::binary::u8.parse_next(input)
}

/// this is a description
fn enc_bar(value: u8) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

fn enc_baz(value: u8) -> Vec<u8> {
    enc_bar(value)
}

fn dec_qux(input: &mut &[u8]) -> winnow::PResult<u8> {
    dec_foo(input)
}

mod nested {
    use super::*;

    pub mod layer {
        use super::*;

        pub mod another {
            use super::*;
            
            pub fn u8_dec(input: &mut &[u8]) -> winnow::PResult<u8> {
                winnow::binary::u8.parse_next(input)
            }
        }
    }
}