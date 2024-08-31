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
mod kst;
mod parse;
mod expand;

#[derive(Error, Debug)]
enum Error {
    #[error("Unable to parse struct attributes for struct `{0}`")]
    UnableToParseStructAttributes(String),
    #[error("`{0}` can only be derived for structs, got `{1}`.")]
    DeriveForNonStruct(String, String),
    #[error("Missing required encoder: `enc = ?`.")]
    MissingEncoder,
    #[error("Missing required decoder: `dec = ?`.")]
    MissingDecoder,
    #[error("Missing required type for default encoder/decoder defined in struct attributes: `#[default(ty = ?)]`.")]
    MissingType,
    #[error("Missing required {0} {1} in struct attributes: `{0}({2} = ?)`.")]
    XcoderIsRequired(String, String, String),

    // #[error("Missing required attribute for `{1}`, function for `{0}`: `#[{0}(func = ?)]`.")]
    // MissingFunc(String, String),
    // #[error("Missing required key for variant `{0}`: `#[key = ?]`.")]
    // MissingKey(String),
    // #[error("Attemping to parse non-integer value for `len` for variant `{1}`: {0}")]
    // NonIntLength(String, String),
    // #[error("Attemping to parse non-byte string for `key` for variant `{1}`: {0}")]
    // NonByteStrKey(String, String),
    // #[error("Encoder type mismatch for variant `{2}`: `#[encoder(typ = {0})]`, but expected {1}.")]
    // EncoderTypeMismatch(String, String, String),
    // #[error("Decoder type mismatch for variant `{2}`: `#[decoder(typ = {0})]`, but expected {1}.")]
    // DecoderTypeMismatch(String, String, String),
}

const NAME: &str = "Klv";
const ATTR: &str = "klv";
#[proc_macro_derive(Klv, attributes(klv))]
pub fn klv(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
}