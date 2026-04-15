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
mod parse;
mod expand;

// mod expand2;
// mod internals;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs, got `{1}`.")]
    DeriveForNonStruct(String, String),
    #[error("Unable to parse struct attributes for struct `{0}`")]
    UnableToParseStructAttributes(String),
    #[error("Missing required type for default encoder/decoder defined in struct attributes: `#[default(ty = ?)]`.")]
    MissingType,
    #[error("Missing required {1} {3} in {0} attributes: `{1}({2} = ?)`.")]
    MissingFunc(String, String, String, String),
    #[error("Missing required key for field `{0}`: `#[key = ?]`.")]
    MissingKey(String),
    #[error("Unable to parse path-like type for enc/dec.")]
    XcoderIsNotPathLike
}

const DERIVE_NAME: &str = "Klv";
const ATTR_NAME: &str = "klv";

#[proc_macro_derive(Klv, attributes(klv, allow))]
pub fn klv_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
}

// #[proc_macro_derive(Klv, attributes(klv, allow))]
// pub fn klv_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     let mut input = parse_macro_input!(input as DeriveInput);
//     expand2::derive(&mut input)
//         .unwrap_or_else(syn::Error::into_compile_error)
//         .into()
// }