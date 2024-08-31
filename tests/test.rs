use tinyklv::Klv;
use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"\x00\x00\x00",
    key(dec = tinyklv::dec::binary::u8,
        enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize,
        enc = tinyklv::enc::binary::u8),
)]
struct Foo {
    #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    name: String,

    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    number: u16,
}

#[test]
fn main() {
    let mut stream1: &[u8] = &[
        // sentinel: 0x00, 0x00, 0x00
        0x00, 0x00, 0x00,
        // packet length: 9 bytes
        0x09,
        // key: 0x01, len: 0x03
        // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
        0x01, 0x03,
        // value decoded: "KLV"
        0x4B, 0x4C, 0x56,
        // key: 0x02, len: 0x02
        // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
        0x02, 0x02,
        // value decoded: 258
        0x01, 0x02,
    ];
    match Foo::extract(&mut stream1) {
        Ok(foo) => {
            assert_eq!(foo.name, "KLV");
            assert_eq!(foo.number, 258);
        },
        Err(e) => panic!("{}", e),
    }
    
    let mut stream2: &[u8] = &[
        // sentinel: 0x00, 0x00, 0x00
        0x00, 0x00, 0x00,
        // packet length: 18 bytes
        0x12,
        // key: 0x01, len: 0x0C
        // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
        0x01, 0x0C,
        // value decoded: "Hello World!"
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21,
        // key: 0x02, len: 0x02
        // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
        0x02, 0x02,
        // value decoded: 42
        0x00, 0x2A,
    ];
    match Foo::extract(&mut stream2) {
        Ok(foo) => {
            assert_eq!(foo.name, "Hello World!");
            assert_eq!(foo.number, 42);
        },
        Err(e) => panic!("{}", e),
    }
}