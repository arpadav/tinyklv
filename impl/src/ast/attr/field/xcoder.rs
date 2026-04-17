// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::{SiguledXcoder, XcoderType};

#[derive(Debug, Default)]
pub(crate) struct FieldXcoder {
    /// The key for the field
    pub key: Option<syn::Lit>,
    /// The encoder for the field, with optional dispatch sigil
    pub enc: Option<SiguledXcoder>,
    /// The decoder for the field
    pub dec: Option<XcoderType>,
    /// Whether the TODO requires a variable length input
    pub var: Option<syn::LitBool>,
    /// syn errors
    pub errors: Option<syn::Error>,

    /// testing: init
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
        let mut var: Option<syn::LitBool> = None;
        let mut init: Option<syn::Expr> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let errors = input
            .parse_nested_meta(|meta| {
                handle_unique_nested_meta_values! {
                    meta;
                    err!(UnknownFieldField(meta.path));
                    5;
                    key: symbol::parse_pnm_key                    => err!(DuplicateKeyInField),
                    enc: symbol::pnm_parse_maybestr_field_encoder => err!(DuplicateEncoderInField),
                    dec: symbol::pnm_parse_maybestr_decoder       => err!(DuplicateDecoderInField),
                    var: symbol::parse_pnm_variable_length  => err!(DuplicateVariableLengthInField),
                    init: symbol::parse_pnm_initial_value,
                }
            })
            .err();
        // println!("init: {:?}", init);
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        FieldXcoder {
            key,
            enc,
            dec,
            var,
            errors,

            init,
        }
    }
}
