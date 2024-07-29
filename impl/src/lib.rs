// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{
    DeriveInput,
    parse_macro_input,
};
use thiserror::Error;

// --------------------------------------------------
// local
// --------------------------------------------------
mod ast;
mod kst;
mod ast2;
mod attr;
mod expand;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs, got `{1}`.")]
    DeriveForNonStruct(String, String),
    #[error("Missing required attribute for `{1}`, function for `{0}`: `#[{0}(func = ?)]`.")]
    MissingFunc(String, String),
    #[error("Missing required attribute for `{1}`, type for `{0}`: `#[{0}(typ = ?)]`.")]
    MissingType(String, String),
    #[error("Missing required key for variant `{0}`: `#[key = ?]`.")]
    MissingKey(String),
    #[error("Missing required length for variant `{0}`: `#[len = ?]`.")]
    MissingLength(String),
    #[error("Attemping to parse non-integer value for `len` for variant `{1}`: {0}")]
    NonIntLength(String, String),
    #[error("Attemping to parse non-byte string for `key` for variant `{1}`: {0}")]
    NonByteStrKey(String, String),
    #[error("Encoder type mismatch for variant `{2}`: `#[encoder(typ = {0})]`, but expected {1}.")]
    EncoderTypeMismatch(String, String, String),
    #[error("Decoder type mismatch for variant `{2}`: `#[decoder(typ = {0})]`, but expected {1}.")]
    DecoderTypeMismatch(String, String, String),
}

// impl<T> Into<nom::IResult<T, T>> for IResult<T> {
//     fn into(self) -> nom::IResult<T, T> {
//         match self {
//             KlvResult::Ok(t) => Ok((t, t)),
//             KlvResult::Err(e) => Err(nom::Err::Error(e)),
//         }
//     }
// }

const NAME: &str = "Klv";
#[proc_macro_derive(Klv, attributes(
    key_encoder,
    key_decoder,
    len_encoder,
    len_decoder,
    default_encoder,
    default_decoder,
    key,
    len,
    encoder,
    decoder
))]
pub fn klv(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // println!("{}: {:#?}=========================================================================", NAME, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    expand::derive(&input).into()
}