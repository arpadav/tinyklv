// --------------------------------------------------
// mods
// --------------------------------------------------
mod xcoder;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;
use xcoder::FieldXcoder;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::container::default::DefaultXcoder;
use crate::ast::types::{SiguledXcoder, XcoderSigil, XcoderType};
use crate::symbol;
use crate::Ctxt;

#[derive(Debug)]
/// Represents field attribute information
pub(crate) struct Field {
    pub contents: FieldXcoder,
}
/// [`Field`] implementation
impl Field {
    /// Extract out the `#[klv(...)]` attributes from a struct field.
    pub fn from_ast(
        cx: &Ctxt,
        field: &syn::Field,
        name: &syn::Ident,
        container_defaults: &HashMap<syn::Type, DefaultXcoder>,
        allow_unimplemented_encode: bool,
        allow_unimplemented_decode: bool,
    ) -> Option<Self> {
        // --------------------------------------------------
        // return if no attrs on field
        // --------------------------------------------------
        if field.attrs.is_empty() {
            return None;
        }

        // --------------------------------------------------
        // loop through attrs
        // --------------------------------------------------
        let mut all_field_xcoders = Vec::new();
        for attr in &field.attrs {
            if attr.path() != symbol::KLV_ATTR {
                continue;
            }
            // --------------------------------------------------
            // parse out klv attributes
            // --------------------------------------------------
            match attr.meta {
                syn::Meta::List(ref contents) => {
                    all_field_xcoders.push(FieldXcoder::from(contents))
                }
                _ => {
                    cx.error_spanned_by(attr, err!(MalformedField));
                }
            }
        }

        // --------------------------------------------------
        // return if no attrs == symbol::KLV_ATTR
        // --------------------------------------------------
        if all_field_xcoders.is_empty() {
            return None;
        }

        // --------------------------------------------------
        // add all `syn_error`'s to `cx`
        // --------------------------------------------------
        all_field_xcoders
            .iter()
            .filter_map(|f| f.errors.clone())
            .for_each(|e| cx.syn_error(e));

        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut field_xcoder = FieldXcoder::default();

        // --------------------------------------------------
        // get all klv attr keys
        // --------------------------------------------------
        let keys = all_field_xcoders
            .iter()
            .filter_map(|f| f.key.clone())
            .collect::<Vec<_>>();
        field_xcoder.key = match keys.len() {
            0 => {
                cx.error_spanned_by(field, err!(MissingKeyInField));
                None
            }
            1 => Some(keys[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateKey));
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr encoders
        // --------------------------------------------------
        let mut keep_enc_none = false;
        let encoders = all_field_xcoders
            .iter()
            .filter_map(|f| f.enc.clone())
            .collect::<Vec<_>>();
        field_xcoder.enc = match encoders.len() {
            0 => None,
            1 => Some(encoders[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateEncoderInField));
                keep_enc_none = true;
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr decoders
        // --------------------------------------------------
        let mut keep_dec_none = false;
        let decoders = all_field_xcoders
            .iter()
            .filter_map(|f| f.dec.clone())
            .collect::<Vec<_>>();
        field_xcoder.dec = match decoders.len() {
            0 => None,
            1 => Some(decoders[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateDecoderInField));
                keep_dec_none = true;
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr var
        // --------------------------------------------------
        let vars = all_field_xcoders
            .iter()
            .filter_map(|f| f.var.clone())
            .collect::<Vec<_>>();
        field_xcoder.var = match vars.len() {
            0 => None,
            1 => Some(vars[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateVariableLengthInField));
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr init's
        // --------------------------------------------------
        let inits = all_field_xcoders
            .iter()
            .filter_map(|f| f.init.clone())
            .collect::<Vec<_>>();
        field_xcoder.init = match inits.len() {
            0 => None,
            1 => Some(inits[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateDecoderInField)); // TODO <-- make custom error
                None
            }
        };

        // --------------------------------------------------
        // set defaults, if no enc/dec was found
        // --------------------------------------------------
        let typ_maybe_unwrapped =
            crate::expand::helpers::unwrap_option_type(&field.ty).unwrap_or(&field.ty);
        if let Some(default) = container_defaults.get(typ_maybe_unwrapped) {
            if let (Some(default_enc), false) = (&default.enc, keep_enc_none) {
                field_xcoder.enc = Some(SiguledXcoder {
                    sigil: XcoderSigil::None,
                    inner: default_enc.clone(),
                });
            }
            if let (Some(default_dec), false) = (&default.dec, keep_dec_none) {
                field_xcoder.dec = Some(default_dec.clone());
                field_xcoder.var = default.var.clone();
            }
        }

        // --------------------------------------------------
        // unimplemented encode error
        // --------------------------------------------------
        if !allow_unimplemented_encode && field_xcoder.enc.is_none() {
            cx.error_spanned_by(
                name.clone(),
                err!(UnimplementedEncode(name, typ_maybe_unwrapped)),
            );
        }

        // --------------------------------------------------
        // unimplemented decode error
        // --------------------------------------------------
        if !allow_unimplemented_decode && field_xcoder.dec.is_none() {
            cx.error_spanned_by(
                name.clone(),
                err!(UnimplementedDecode(name, typ_maybe_unwrapped)),
            );
        }

        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Some(Field {
            contents: field_xcoder,
        })
    }
}

/// A parsed field
pub(crate) struct FieldParsed {
    pub key: syn::Lit,
    pub enc: Option<SiguledXcoder>,
    pub dec: Option<XcoderType>,
    pub var: Option<syn::LitBool>,

    pub _init: Option<syn::Expr>,
}
/// [`FieldParsed`] implementation
impl FieldParsed {
    pub fn from_field(cx: &Ctxt, sf: &syn::Field, f: &Field) -> Option<FieldParsed> {
        // --------------------------------------------------
        // check for required fields
        // --------------------------------------------------
        let key = match &f.contents.key {
            Some(k) => k,
            None => {
                cx.error_spanned_by(sf.ident.clone(), err!(MissingKeyInField));
                return None;
            }
        };
        let _init = f.contents.init.clone();
        // --------------------------------------------------
        // return parsed field
        // --------------------------------------------------
        Some(FieldParsed {
            key: key.clone(),
            enc: f.contents.enc.clone(),
            dec: f.contents.dec.clone(),
            var: f.contents.var.clone(),

            _init: f.contents.init.clone(),
        })
    }
}
