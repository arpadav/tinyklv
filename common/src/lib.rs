// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;

// --------------------------------------------------
// local
// --------------------------------------------------
pub mod symple;

#[derive(Const)]
#[armtype(&str)]
/// Struct Attribute Names
pub enum StructNames {
    /// The stream type. Defaults to &[u8]
    #[value = "stream"]
    Stream,
    /// The sentinel value. Defaults to `None`
    #[value = "sentinel"]
    Sentinel,
    /// The key xcoder tuple
    #[value = "key"]
    KeyTuple,
    /// The length xcoder tuple
    #[value = "len"]
    LengthTuple,
    /// The default xcoder tuple
    #[value = "default"]
    DefaultTuple,
}

#[derive(Const)]
#[armtype(&str)]
/// Xcoder Names
pub enum XcoderNames {
    #[value = "ty"]
    /// The type associated with the encoder and decoder
    Type,
    #[value = "dyn"]
    /// Determines whether or not the length is dynamically determined
    DynLen,
    #[value = "enc"]
    /// The encoder, a function which is `winnow` compatible
    Encoder,
    #[value = "dec"]
    /// The decoder, a function which is `winnow` compatible
    Decoder,
}

#[derive(Const)]
#[armtype(&str)]
/// Field Attribute Names
pub enum FieldNames {
    #[value = "key"]
    /// The key. Required: as a slice of `stream` type.
    /// 
    /// This is a required attribute, written using a literal (either bytes
    /// or str), to help identify the field during parsing.
    /// 
    /// Non-literal keys are currently not supported.
    Key,

    #[value = "dyn"]
    /// The dynamic length. Optional: defaults to `false`.
    /// 
    /// This is an optional attribute, which indicates the length of the field is dynamic.
    /// This is commonly used for Strings, but can be for other values as well. 
    /// 
    /// For example, if the field is of type [u8], it will almost always be a single byte which is 
    /// parsed as the length. For [u16], it will be two bytes. This indicates a **constant** length, 
    /// therefore the `dyn` length keyword can be omitted since the parser used will never use
    /// the input length.
    /// 
    /// In practice, streams would look like:
    /// 
    /// ```rust
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(
    ///     sentinel = b"\x00\x00\x00",
    ///     key(dec = tinyklv::dec::binary::u8,
    ///         enc = tinyklv::enc::binary::u8),
    ///     len(dec = tinyklv::dec::binary::u8_as_usize,
    ///         enc = tinyklv::enc::binary::u8),
    /// )]
    /// struct Foo {
    ///     #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    ///     name: String,
    /// 
    ///     #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    ///     number: u16,
    /// }
    /// 
    /// fn main() {
    ///     let mut stream1: &[u8] = &[
    ///         // sentinel: 0x00, 0x00, 0x00
    ///         0x00, 0x00, 0x00,
    ///         // packet length: 9 bytes
    ///         0x09,
    ///         // key: 0x01, len: 0x03
    ///         // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    ///         0x01, 0x03,
    ///         // value decoded: "KLV"
    ///         0x4B, 0x4C, 0x56,
    ///         // key: 0x02, len: 0x02
    ///         // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    ///         0x02, 0x02,
    ///         // value decoded: 258
    ///         0x01, 0x02,
    ///     ];
    ///     match Foo::decode(&mut stream1) {
    ///         Ok(foo) => {
    ///             assert_eq!(foo.name, "KLV");
    ///             assert_eq!(foo.number, 258);
    ///         },
    ///         Err(e) => panic!("{}", e),
    ///     }
    ///     
    ///     let mut stream2: &[u8] = &[
    ///         // sentinel: 0x00, 0x00, 0x00
    ///         0x00, 0x00, 0x00,
    ///         // packet length: 18 bytes
    ///         0x12,
    ///         // key: 0x01, len: 0x0C
    ///         // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    ///         0x01, 0x0C,
    ///         // value decoded: "Hello World!"
    ///         0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21,
    ///         // key: 0x02, len: 0x02
    ///         // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    ///         0x02, 0x02,
    ///         // value decoded: 42
    ///         0x00, 0x2A,
    ///     ];
    ///     match Foo::decode(&mut stream2) {
    ///         Ok(foo) => {
    ///             assert_eq!(foo.name, "Hello World!");
    ///             assert_eq!(foo.number, 42);
    ///         },
    ///         Err(e) => panic!("{}", e),
    ///     }
    /// }
    /// ```
    DynLen,

    #[value = "enc"]
    /// The encoder
    Encoder,

    #[value = "dec"]
    /// The decoder]
    Decoder,
}