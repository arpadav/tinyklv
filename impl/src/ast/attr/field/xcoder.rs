// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::{LatebindXcoder, SiguledXcoder, XcoderType};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug, Default)]
pub(crate) struct FieldXcoder {
    /// The key for the field
    pub key: Option<syn::Lit>,
    /// The encoder for the field, with optional dispatch sigil
    pub enc: Option<SiguledXcoder>,
    /// The decoder for the field
    pub dec: Option<XcoderType>,
    /// Whether the decoder requires a variable length input
    pub varlen: Option<syn::LitBool>,
    /// Post-decode conversion or in-place mutation (`latebind = path` or
    /// `latebind = &mut path`). `None` means no post-decode step.
    pub latebind: Option<LatebindXcoder>,
    /// syn errors
    pub errors: Option<syn::Error>,
    /// init
    pub init: Option<syn::Expr>,
}
/// [`FieldXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
impl From<&syn::MetaList> for FieldXcoder {
    // type Error = syn::Error;
    fn from(input: &syn::MetaList) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut key: Option<syn::Lit> = None;
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut varlen: Option<syn::LitBool> = None;
        let mut latebind: Option<LatebindXcoder> = None;
        let mut init: Option<syn::Expr> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let errors = input
            .parse_nested_meta(|meta| {
                handle_unique_nested_meta_values! {
                    meta;
                    err!(UnknownFieldField(meta.path));
                    6;
                    key: symbol::parse_pnm_key                      => err!(DuplicateKeyInField),
                    enc: symbol::pnm_parse_maybestr_encoder         => err!(DuplicateEncoderInField),
                    dec: symbol::pnm_parse_maybestr_decoder         => err!(DuplicateDecoderInField),
                    varlen: symbol::parse_pnm_variable_length       => err!(DuplicateVariableLengthInField),
                    latebind: symbol::pnm_parse_maybestr_latebind   => err!(DuplicateLatebindInField),
                    init: symbol::parse_pnm_initial_value,
                }
            })
            .err();
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        FieldXcoder {
            key,
            enc,
            dec,
            varlen,
            latebind,
            errors,
            init,
        }
    }
}
