//! Encode traits for KLV field values and key-length-value framing
//!
//! Core encode-side traits:
//! * [`EncodeValue`] - appends the encoded value portion of a KLV triple to a caller-owned buffer
//! * [`EncodeFrame`] - appends a full key-length-value byte sequence to a caller-owned buffer
//!
//! Decode counterparts live in [`crate::traits::dec`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
pub use super::*;

/// Appends the encoded value portion of a KLV field to a caller-owned [`Vec<u8>`]
///
/// Decode counterpart: [`DecodeValue`](crate::traits::DecodeValue)
///
/// Trait for encoding ***data only*** by appending its bytes to an output buffer. Writing
/// into a caller-owned buffer (rather than returning a fresh allocation) lets one buffer be
/// reused across many records and lets a field's value be written without a per-field heap alloc
///
/// ```text
///                                  This is what is encoded
///                                 vvvvvvvvvvvvvvvvvvvvvvvvvv
/// [ ... key ... | ... length ... | ..... value (self) ..... ]
///                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
///
/// ***This trait IS automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait, in which every field has an associated encoder for it's type. Otherwise, this trait CAN be implemented manually.***
///
/// For custom encoding functions, ***no need to use this trait***. Instead, ensure the function
/// appends the value bytes to the buffer with a signature matching the following:
///
/// ```rust ignore
/// fn encoder_fn_name(input: &T, out: &mut Vec<u8>);
/// ```
///
/// # Example
///
/// ```rust
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
/// use tinyklv::traits::EncodeValue;
///
/// struct InnerValue {}
///
/// fn ex01_encoder(input: &InnerValue, out: &mut Vec<u8>) {
///     out.extend_from_slice(&[0x65, 0x66, 0x67, 0x68]);
/// }
///
/// fn ex02_encoder(input: &InnerValue, out: &mut Vec<u8>) {
///     out.extend_from_slice(&String::from("Y2K").into_bytes());
/// }
///
/// impl EncodeValue for InnerValue {
///     fn encode_value(&self, out: &mut Vec<u8>) {
///         out.extend_from_slice(&String::from("KLV").to_lowercase().into_bytes());
///     }
/// }
///
/// #[derive(Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = b"\x00",
///     key(enc = tinyklv::codecs::binary::enc::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8_from_usize),
/// )]
/// #[klv(allow_unimplemented_decode)]
/// struct MyStruct {
///     #[klv(key = 0x07, enc = ex01_encoder)]
///     example_one: InnerValue,
///
///     #[klv(key = 0x0A, enc = ex02_encoder)]
///     example_two: InnerValue,
///
///     #[klv(key = 0x8A, enc = InnerValue::encode_value)]
///     example_three: InnerValue,
/// }
///
/// let mut my_struct_value_encoded = Vec::new();
/// MyStruct {
///     example_one: InnerValue {},
///     example_two: InnerValue {},
///     example_three: InnerValue {},
/// }.encode_value(&mut my_struct_value_encoded);
///
/// assert_eq!(my_struct_value_encoded, vec![
///     // example 1
///     0x07,               // example 1 key
///     0x04,               // example 1 length
///                         // example 1 value
///     0x65, 0x66, 0x67, 0x68,
///
///     // example 2
///     0x0A,               // example 2 key
///     0x03,               // example 2 length
///     0x59, 0x32, 0x4B,   // example 2 value
///
///     // example 3
///     0x8A,               // example 3 key
///     0x03,               // example 3 length
///     0x6B, 0x6C, 0x76,   // example 3 value
/// ]);
///
/// let mut my_struct_encoded = Vec::new();
/// MyStruct {
///     example_one: InnerValue {},
///     example_two: InnerValue {},
///     example_three: InnerValue {},
/// }.encode_frame(&mut my_struct_encoded); // See: `tinyklv::prelude::EncodeFrame` -> This prepends the key and length
///
/// assert_eq!(my_struct_encoded, vec![
///     0x00,               // sentinel
///     0x10,               // total length
///
///     // example 1
///     0x07,               // example 1 key
///     0x04,               // example 1 length
///                         // example 1 value
///     0x65, 0x66, 0x67, 0x68,
///
///     // example 2
///     0x0A,               // example 2 key
///     0x03,               // example 2 length
///     0x59, 0x32, 0x4B,   // example 2 value
///
///     // example 3
///     0x8A,               // example 3 key
///     0x03,               // example 3 length
///     0x6B, 0x6C, 0x76,   // example 3 value
/// ]);
/// ```
pub trait EncodeValue {
    /// Appends the encoded value portion of this KLV field to `out`
    ///
    /// Writes **only** the value bytes (no key or length prefix) to the end of the
    /// caller-owned buffer, growing it as needed. Use [`EncodeFrame::encode_frame`]
    /// to write a complete key-length-value triple instead
    ///
    /// # Arguments
    ///
    /// * `out` - The output buffer the value bytes are appended to
    fn encode_value(&self, out: &mut Vec<u8>);
}

/// [`EncodeValue`] for any `Vec<T>` whose element type encodes - the encode twin of
/// [`DecodeValue for Vec<T>`](crate::traits::DecodeValue)
///
/// Appends every element's encoded value back-to-back into `out`, with no framing between elements.
/// Because each element is written by `T::encode_value`, this is correct **only when `T` is
/// self-delimiting** - a fixed-width or otherwise self-terminating value that the matching
/// `Vec<T>` decode can split back apart. A `#[derive(Klv)]` struct's `encode_value` writes its body
/// with no outer length, so a `Vec` of derived records would not round-trip (the first element's
/// decode would consume the whole run); give such elements a self-delimiting hand-written codec
///
/// Note: there is no `EncodeValue for u8` (primitives encode via the `codecs` free functions), so
/// `Vec<u8>` does not resolve through this blanket and keeps its existing path - adding an
/// `EncodeValue for u8` in future would silently change that
impl<T> EncodeValue for Vec<T>
where
    T: EncodeValue,
{
    #[inline]
    fn encode_value(&self, out: &mut Vec<u8>) {
        self.iter().for_each(|item| item.encode_value(out));
    }
}

/// Full KLV encode pipeline: prepends key and length to [`EncodeValue`] output
///
/// Decode counterpart: [`DecodeFrame`](crate::traits::DecodeFrame)
///
/// Trait for encoding data to its full key-length-value representation
///
/// ```text
///                 This is what is encoded
///  vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
/// [ ... key ... | ... length ... | ..... value (self) ..... ]
///  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
///
/// ***This trait IS automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait, in which every field has an associated encoder for it's type. Otherwise, this trait CAN be implemented manually.***
///
/// Since this function and [`EncodeValue`] have the same function signature, this could cause confusion. Therefore, the general workflow for all KLV encoding should be:
///
/// 1. Encode the struct/value
///
/// This can be done by implementing the [`EncodeValue`] trait
///
/// 2. Prepend the encoded value with its key/recognition sentinel and length to convert the struct/value into its key-length-value representation
///
/// Then, youre done: now you can produce the key-length-value representation of your struct with the following snippet:
///
/// ```rust
/// use tinyklv::prelude::*;
/// use tinyklv::traits::EncodeValue;
///
/// struct MyStruct {}
///
/// impl EncodeValue for MyStruct {
///     fn encode_value(&self, out: &mut Vec<u8>) {
///         out.extend_from_slice("example".as_bytes());
///     }
/// }
///
/// let my_struct = MyStruct {};
///
/// let key_len_val_of_my_struct = {
///     let mut out = Vec::new();
///     let mut body = Vec::new();
///     my_struct.encode_value(&mut body);
///     out.extend_from_slice(&[0xFF, 0xBB]);            // encoded key
///     out.extend_from_slice(&(body.len() as u8).to_be_bytes()); // length encoder
///     out.extend_from_slice(&body);
///     out
/// };
///
/// assert_eq!(key_len_val_of_my_struct, [
///     0xFF, 0xBB,                         // key
///     0x07,                               // length
///     101, 120, 97, 109, 112, 108, 101,   // value
/// ]);
/// ```
///
/// However, you can also implement the [`EncodeFrame`] trait instead to combine both of these operations:
///
/// ```rust
/// use tinyklv::prelude::*;
/// use tinyklv::traits::{EncodeValue, EncodeFrame};
///
/// struct MyStruct {}
///
/// impl EncodeValue for MyStruct {
///     fn encode_value(&self, out: &mut Vec<u8>) {
///         out.extend_from_slice("example".as_bytes());
///     }
/// }
///
/// impl EncodeFrame for MyStruct {
///     fn encode_frame(&self, out: &mut Vec<u8>) {
///         let mut body = Vec::new();
///         self.encode_value(&mut body);
///         out.extend_from_slice(&[0xFF, 0xBB]);            // encoded key
///         out.extend_from_slice(&(body.len() as u8).to_be_bytes()); // length encoder
///         out.extend_from_slice(&body);
///     }
/// }
///
/// let my_struct = MyStruct {};
///
/// let mut key_len_val_of_my_struct = Vec::new();
/// my_struct.encode_frame(&mut key_len_val_of_my_struct);
///
/// assert_eq!(key_len_val_of_my_struct, [
///     0xFF, 0xBB,                         // key
///     0x07,                               // length
///     101, 120, 97, 109, 112, 108, 101,   // value
/// ]);
/// ```
///
/// Furthermore, you can use the [`crate::Klv`] macro to implement both of these for you. (Note that the examples above on a blank struct aren't representative of how the macro works, so a field has been added to the struct):
///
/// ```rust
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
///
/// fn string_encoder(input: &String, out: &mut Vec<u8>) {
///     out.extend_from_slice(input.as_bytes());
/// }
///
/// #[derive(Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = b"\x00",
///     allow_unimplemented_decode,
///     key(enc = tinyklv::codecs::binary::enc::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8_from_usize),
/// )]  // this implements `tinyklv::prelude::EncodeValue` and `tinyklv::prelude::EncodeFrame`
///     // given a key and length encoder are provided
/// struct MyStruct {
///     #[klv(key = 0xFF, enc = string_encoder)]
///     value: String,
/// }
///
/// // using `tinyklv::Klv` to encode as KLV
/// let mut mystruct_klv_1 = Vec::new();
/// MyStruct {
///     value: "example".into()
/// }.encode_frame(&mut mystruct_klv_1);                // `tinyklv::prelude::EncodeFrame` implementation
///
/// // using manual implementation to encode as KLV
/// let mut mystruct_klv_2 = Vec::new();
/// {
///     let mut body = Vec::new();
///     MyStruct {
///         value: "example".into()
///     }.encode_value(&mut body);
///     mystruct_klv_2.extend_from_slice(&[0x00]);      // recognition sentinel
///     tinyklv::codecs::binary::enc::u8_from_usize(body.len(), &mut mystruct_klv_2); // length encoder
///     mystruct_klv_2.extend_from_slice(&body);
/// }                                                   // this is now equivalent to the macro call
///
/// assert_eq!(mystruct_klv_1, mystruct_klv_2);
/// ```
pub trait EncodeFrame {
    /// Appends `self` as a complete KLV frame to `out`: key bytes, then length bytes, then value bytes
    ///
    /// Writes a full key-length-value triple to the end of the caller-owned buffer in
    /// one pass. The derive macro generates this implementation when both `key` and
    /// `len` encoders are provided in the `#[klv(...)]` attribute
    ///
    /// # Arguments
    ///
    /// * `out` - The output buffer the framed bytes are appended to
    fn encode_frame(&self, out: &mut Vec<u8>);
}
