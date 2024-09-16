// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;

// --------------------------------------------------
// local
// --------------------------------------------------
pub mod symple;

#[derive(Const, Debug)]
#[armtype(&str)]
/// Struct attributes for `tinyklv` and their input arguments
/// 
/// # Syntax
/// 
/// ```no_run ignore
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
/// 
/// #[derive(Klv)]
/// #[klv(
///     // key / value pairs
///     <strct-attr> = <value>,
///     // tuples
///     <strct-attr>(<attr> = <value>, ...),
///     ...
/// )]
/// struct <STRUCTNAME> { ... }
/// ```
pub enum StructNames {
    #[value = "stream"]
    /// `stream` ***(Optional)***: The type of data being streamed in
    /// 
    /// Usually `&[u8]` or `&str`.
    /// 
    /// # Syntax
    /// 
    /// `stream = <type>`
    ///
    /// # Defaults to
    /// 
    /// `&[u8]`
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(stream = &[u8], ...)]`
    /// * `#[klv(stream = &str, ...)]`
    /// 
    /// In practice, setting the stream would look like:
    /// 
    /// ```no_run ignore
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(stream = &[u8], ...)]
    /// struct Foo { ... }
    /// ```
    Stream,

    #[value = "sentinel"]
    /// `sentinel` ***(Optional)***: The recognition sentinel / universal header value
    /// 
    /// When using a sentinel, it assumes that the stream starts with the
    /// sentinel value and is followed by the length of the remaining data within
    /// the packet. With the sentinel, it is recommended to use the `extract` method
    /// (see [`tinyklv::prelude::Extract`](https://docs.rs/tinyklv/latest/tinyklv/prelude/extract/index.html))
    /// which performs a seek and then a decode.
    /// 
    /// The [`tinyklv::prelude::Decode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/decode/index.html) method
    /// only decodes the data which follows the sentinel, not the header itself.
    /// 
    /// When the sentinel is not set, it is assumed that the user is handling
    /// the entire stream ingestion and is only using the `decode` method for parsing of the
    /// data.
    /// 
    /// # Syntax
    /// 
    /// `sentinel = <literal>`
    /// 
    /// # Defaults to
    /// 
    /// None
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(sentinel = b"\x00\x00\x00", ...)]`
    /// * `#[klv(sentinel = b"my_packet_starts_with_this_message", ...)]`
    /// 
    /// In practice, setting the sentinel would look like:
    /// 
    /// ```no_run ignore
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(sentinel = b"\x00\x00\x00", ...)]
    /// struct Foo { ... }
    /// ```
    Sentinel,

    #[value = "key"]
    /// `key` ***(Required)***: The decoder/encoder used for parsing/generating the key
    /// 
    /// This describes the function used to encode the key into stream `S`, and
    /// the function used to decode the key from stream `S`.
    /// 
    /// One of the `enc` or `dec` attributes must be set for traits to be automatically
    /// generated for encoding or decoding of the stream. ***If neither are set,
    /// then the proc-macro will not generate any code.***
    /// 
    /// # Syntax
    /// 
    /// `key(enc = <path-to-encoder>, dec = <path-to-decoder>)`
    /// 
    /// Used args:
    /// 
    /// * [`XcoderNames::Encoder`] (Optional)
    /// * [`XcoderNames::Decoder`] (Optional)
    /// 
    /// Unused args:
    /// 
    /// * [`XcoderNames::Type`] - Type is automatically determined based off of `stream` type
    /// as well as the value of the `key` attributes for each field of the struct.
    /// * [`XcoderNames::DynLen`] - This flag is only used for encoders/decoders for
    /// ***values***. For the encoders/decoders for the keys and lengths itself, this
    /// flag is unused.
    /// 
    /// # Example usage
    /// 
    /// Please refer to [`XcoderNames::Encoder`] and [`XcoderNames::Decoder`] for 
    /// example usage for setting the `enc` and `dec` arguments.
    KeyTuple,

    #[value = "len"]
    /// `len` ***(Required)***: The decoder/encoder used for parsing/generating the length
    /// 
    /// This describes the function used to encode the length into stream `S`, and
    /// the function used to decode the length from stream `S`.
    /// 
    /// One of the `enc` or `dec` attributes must be set for traits to be automatically
    /// generated for encoding or decoding of the stream. ***If neither are set,
    /// then the proc-macro will not generate any code.***
    /// 
    /// # Syntax
    /// 
    /// `len(enc = <path-to-encoder>, dec = <path-to-decoder>)`
    /// 
    /// Used args:
    /// 
    /// * [`XcoderNames::Encoder`] (Optional)
    /// * [`XcoderNames::Decoder`] (Optional)
    /// 
    /// Unused args:
    /// 
    /// * [`XcoderNames::Type`] - Type is automatically determined based off of `stream` type
    /// as well as the value of the `len` attributes for each field of the struct.
    /// * [`XcoderNames::DynLen`] - This flag is only used for encoders/decoders for
    /// ***values***. For the encoders/decoders for the keys and lengths itself, this
    /// flag is unused.
    /// 
    /// # Example usage
    /// 
    /// Please refer to [`XcoderNames::Encoder`] and [`XcoderNames::Decoder`] for 
    /// example usage for setting the `enc` and `dec` arguments.
    LengthTuple,

    #[value = "default"]
    /// `default` ***(Optional)***: The default decoder/encoder for a specified type
    /// 
    /// This describes the function used to encode values of stream `S` for a default type
    /// and the function used to decode values from stream `S` for a default type.
    /// 
    /// One of the `enc` or `dec` attributes must be set for traits to be automatically
    /// generated for encoding or decoding of the stream. ***If neither are set,
    /// then this does nothing.***
    /// 
    /// # Syntax
    /// 
    /// `default(ty = <type>, enc = <path-to-encoder>, dec = <path-to-decoder>)`
    /// 
    /// Used args:
    /// 
    /// * [`XcoderNames::Type`] (Required)
    /// * [`XcoderNames::Encoder`] (Optional)
    /// * [`XcoderNames::Decoder`] (Optional)
    /// 
    /// Unused args:
    /// 
    /// * [`XcoderNames::DynLen`] - This flag is only used for encoders/decoders for
    /// ***values***. For the encoders/decoders for the keys and lengths itself, this
    /// 
    /// # Example usage
    /// 
    /// Please refer to [`XcoderNames::Encoder`] and [`XcoderNames::Decoder`] for 
    /// example usage for setting the `enc` and `dec` arguments.
    DefaultTuple,

    #[value = "allow_unimplemented_decode"]
    AllowUnimplementedDecode,

    #[value = "allow_unimplemented_encode"]
    AllowUnimplementedEncode,
}

#[derive(Const)]
#[armtype(&str)]
/// Encoder / decoder attribute arguments
/// 
/// For shortening, the term "xcoder" is used throughout the documentation and code base
/// to describe either encoder or decoder.
/// 
/// # Syntax
/// 
/// 
pub enum XcoderNames {
    #[value = "ty"]
    /// `ty` ***(Required for `default`)***: The type associated with the encoder / decoder
    /// 
    /// This describes the type of the xcoder. For key/length xcoder's, the type is implied:
    /// 
    /// * `key` type: slice of stream type `S`
    /// * `len` type: Always [`usize`]
    /// 
    /// And for struct field items, the type is implied by the attribute field:
    /// 
    /// ```ignore no_run
    /// #[klv(key = 0x01, dec = ...)]
    /// item: String,           // <-- type is String
    /// 
    /// #[klv(key = 0x02, dec = ...)]
    /// item: Option<String>,   // <-- type is String
    /// ```
    /// 
    /// For `default` xcoders, if no `enc` or `dec` is 
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
/// Field attributes for `tinyklv` and their input arguments
/// 
/// # Syntax
/// 
/// ```no_run ignore
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
/// 
/// #[derive(Klv)]
/// #[klv(...)]
/// struct <STRUCTNAME> {
///     #[klv(
///         <field-attr> = <value>,
///         ...
///     )]
///     <field>: <ty>,
///     ...
/// }
/// ```
pub enum FieldNames {
    #[value = "key"]
    /// `key` ***(Required)***: The key associated with the field
    /// 
    /// # Syntax
    /// 
    /// `key = <literal>`
    /// 
    /// # Value
    /// 
    /// The literal value should be a slice of `stream` type. Usually `&[u8]` 
    /// or `&str`. This is a required attribute, written using a literal, to
    /// help identify the field during parsing.
    /// 
    /// Non-literal keys are currently not supported.
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(key = 0x01, ...)]`
    /// * `#[klv(key = "foo", ...)]`
    /// * `#[klv(key = b"\x01\x02\x03", ...)]`
    Key,

    #[value = "dyn"]
    /// `dyn` ***(Optional)***: Indicates a field of dynamic length
    /// 
    /// This is commonly set to true for [`String`], but can be for other values as well. 
    /// 
    /// For example, if the field is of type [`u16`], it will almost always be of ***constant*** length
    /// of two bytes. Therefore, instead of calling a decoder/parser with signature:
    /// 
    /// ```no_run ignore
    /// // dyn = true
    /// fn parse_u16(input: &mut &[u8], length: usize) -> winnow::PResult<u16>;
    /// ```
    /// 
    /// It can be written as:
    /// 
    /// ```no_run ignore
    /// // dyn = false
    /// fn parse_u16(input: &mut &[u8]) -> winnow::PResult<u16>
    /// // Implied length is 2
    /// ```
    /// 
    /// Since the parser will never use the length, it can be omitted.
    /// 
    /// # Syntax
    /// 
    /// `dyn = <bool>`
    /// 
    /// # Value
    /// 
    /// The literal value should be `true` or `false`.
    /// 
    /// # Defaults to
    /// 
    /// `false`
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(dyn = true, ...)]`
    /// * `#[klv(dyn = false, ...)]`
    /// 
    /// In practice, streams would look like:
    /// 
    /// ```no_run ignore
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
    ///     default(ty = Vec<f64>, dyn = true, dec = crate::another_dynamic_decoder),
    ///     default(ty = u64, dec = tinyklv::dec::binary::be_u64),
    /// )]
    /// struct Foo {
    ///     #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    ///     name: String,
    /// 
    ///     #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    ///     number: u16,
    /// }
    /// ```
    DynLen,

    #[value = "dec"]
    /// `dec` ***(Optional)***: The decoder associated with the field
    /// 
    /// A path to the decoder function with signature:
    /// 
    /// ```no_run ignore
    /// // dyn = false, length is implied
    /// fn dec<T>(input: &mut S) -> winnow::PResult<T>;
    /// ```
    /// 
    /// OR
    /// 
    /// ```no_run ignore
    /// // dyn = true, length is read from stream
    /// fn dec<T>(input: &mut S, length: usize) -> winnow::PResult<T>;
    /// ```
    /// 
    /// Where `S` is the type of the stream and `T` is the type of the
    /// field.
    /// 
    /// For example, `tinyklv::dec::binary::be_u16` will decode
    /// a big endian [`u16`] from a stream of type `&[u8]`.
    /// 
    /// # Syntax
    /// 
    /// `dec = <path-to-decoder>`
    /// 
    /// # Value
    /// 
    /// See description
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(dec = tinyklv::dec::binary::be_u16)]`
    /// * `#[klv(dec = tinyklv::dec::binary::be_u16_as_usize)]`
    /// 
    /// In practice, streams would look like:
    /// 
    /// ```no_run ignore
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(...)]
    /// struct Foo {
    ///     #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    ///     name: String,
    /// 
    ///     #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    ///     number: u16,
    /// }
    /// ```
    Decoder,

    #[value = "enc"]
    /// `enc` ***(Optional)***: The encoder associated with the field
    /// 
    /// A path to the encoder function with signature:
    /// 
    /// ```no_run ignore
    /// fn enc<S>(input: T) -> Owned<S>;
    /// ```
    /// 
    /// Where `T` is the type of your struct element, and `S` is the type
    /// of the stream.
    /// 
    /// For example, `tinyklv::enc::binary::be_u16` will encode
    /// a [`u16`] as big endian bytes into a owned [`Vec<u8>`], which can
    /// be referenced as a slice for stream type `&[u8]`.
    /// 
    /// # Syntax
    /// 
    /// `enc = <path-to-encoder>`
    /// 
    /// # Value
    /// 
    /// See description
    /// 
    /// # Example usage
    /// 
    /// * `#[klv(enc = tinyklv::enc::binary::be_u16)]`
    /// * `#[klv(enc = tinyklv::enc::binary::le_u16)]`
    /// * `#[klv(enc = tinyklv::enc::binary::be_u32)]`
    /// 
    /// In practice, streams would look like:
    /// 
    /// ```no_run ignore
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(...)]
    /// struct Foo {
    ///     #[klv(key = 0x01, dyn = true, enc = tinyklv::enc::binary::to_string)]
    ///     name: String,
    /// 
    ///     #[klv(key = 0x02, enc = tinyklv::enc::binary::be_u16)]
    ///     number: u16,
    /// }
    /// ```
    Encoder,
}