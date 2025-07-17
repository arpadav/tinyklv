// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::{TypeType, XcoderType};

#[derive(Debug)]
/// A default encoder / decoder for a specific type
/// 
/// # Syntax
/// 
/// `default(typ = <type>, enc = <path>, dec = <path>, var = <bool>)`
/// 
/// Both `enc` and `dec` are optional, however at least one must be provided.
/// 
/// `var` defaults to `false`.
pub(crate) struct DefaultXcoder {
    /// The type associated with the encoder / decoder
    pub typ: Option<TypeType>,
    /// The encoder
    pub enc: Option<XcoderType>,
    /// The decoder
    pub dec: Option<XcoderType>,
    /// Whether the decoder requires a variable length input
    pub var: Option<syn::LitBool>,
    /// syn errors
    pub errors: Option<syn::Error>,
}
/// [`DefaultXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
impl From<&syn::MetaList> for DefaultXcoder {
    fn from(input: &syn::MetaList) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut typ: Option<TypeType> = None;
        let mut enc: Option<XcoderType> = None;
        let mut dec: Option<XcoderType> = None;
        let mut var: Option<syn::LitBool> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let mut errors = input.parse_nested_meta(|meta| {
            let maybe_typ = typ
                .clone()
                .map(|typ| typ.to_token_stream().to_string())
                .unwrap_or(String::from("not defined"));
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownDefaultField(meta.path));
                4;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInDefault(maybe_typ)),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInDefault(maybe_typ)),
                typ: symbol::pnm_parse_maybestr_type    => err!(DuplicateTypeInDefault),
                var: symbol::parse_pnm_variable_length  => err!(DuplicateVariableLengthInDefault),
            }
        }).err();
        // --------------------------------------------------
        // at least one encoder or one decoder must be provided
        // --------------------------------------------------
        if enc.is_none() && dec.is_none() {
            let e = syn::Error::new_spanned(
                input.clone(),
                err!(MissingEncDecInDefault),
            );
            match errors {
                None => errors = Some(e),
                Some(ref mut errors) => errors.combine(e),
            }
        }
        // --------------------------------------------------
        // type must be provided
        // --------------------------------------------------
        if typ.is_none() {
            let e = syn::Error::new_spanned(
                input.clone(),
                err!(MissingTypeInDefault),
            );
            match errors {
                None => errors = Some(e),
                Some(ref mut errors) => errors.combine(e),
            }
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        DefaultXcoder {
            typ,
            enc,
            dec,
            var,
            errors,
        }
    }
}