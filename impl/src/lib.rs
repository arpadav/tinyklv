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
mod attr;
mod expand;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs.")]
    DeriveForNonStruct(String),
    #[error("Missing required attribute: function in `#[{0}(func = ?)]`.")]
    MissingFunc(String),
    #[error("Missing required attribute: type in `#[{0}(typ = ?)]`.")]
    MissingType(String),
    #[error("Attemping to parse non-integer value for `len`: {0}")]
    NonIntLength(String),
    #[error("Attemping to parse non-byte string for `key`: {0}")]
    NonByteStrKey(String),
    #[error("Encoder type mismatch: `#[encoder(typ = {0})]`, but expected {1} from variant `{2}`.")]
    EncoderTypeMismatch(String, String, String),
    #[error("Decoder type mismatch: `#[decoder(typ = {0})]`, but expected {1} from variant `{2}`.")]
    DecoderTypeMismatch(String, String, String),
}

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