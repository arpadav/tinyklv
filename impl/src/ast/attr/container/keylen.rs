// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::XcoderType;

#[derive(Debug, Clone)]
/// A key or length encoder/decoder
/// 
/// Unimplemented encoding/decoding could be allowed, so both
/// encoder and decoder are optional.
pub struct Xcoder {
    /// The encoder. Is optional, since unimplemented encoding could be allowed
    pub enc: Option<XcoderType>,
    /// The decoder. Is optiona, since unimplemented decoding could be allowed
    pub dec: Option<XcoderType>,
}
/// [`Xcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
impl TryFrom<&syn::MetaList> for Xcoder {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaList) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut enc: Option<XcoderType> = None;
        let mut dec: Option<XcoderType> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            tk_syn_macros::handle_unique_nested_meta_values! {
                meta;
                err!(UnknownKeyLenField(meta.path));
                2;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInKeyLen),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInKeyLen),
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(Xcoder { enc, dec })
    }
}