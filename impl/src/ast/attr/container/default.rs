//! Parsing of the `default(..)` container-level attribute
//!
//! Defines [`DefaultXcoder`], which represents one entry in a `#[klv(default(..))]`
//! annotation on a struct, and then every entry of its type merged. Each entry supplies a type
//! together with an optional encoder, decoder, size and skip, each used for any field of that
//! type that leaves it unset; the size's `lengthed` goes with the decoder
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Ctxt;
use crate::ast::attr::{size::SizeSpec, skip_if::SkipIf};
use crate::ast::symbol;
use crate::ast::types::{SiguledXcoder, TypeType, XcoderType};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug)]
/// A default encoder / decoder for a specific type
///
/// # Syntax
///
/// `default(typ = <type>, enc = <path>, dec = <path>,
/// size(implicit, lengthed, exact = N | reserve = N), skip_if_default | skip_if = <path>)`
///
/// Both `enc` and `dec` are optional, however the entries of one type must provide at least one
///
/// `size` is optional; a matched field takes its `exact` / `reserve` where the field's own
/// `size(..)` has neither, its `implicit` where the field's own has none, and its `lengthed`
/// when the field uses this `dec`. A `lengthed` requires a `dec` among the entries of its type
///
/// The skip is optional; a matched field takes it unless it sets its own
pub(crate) struct DefaultXcoder {
    /// The type associated with the encoder / decoder
    pub typ: Option<TypeType>,

    /// The encoder
    pub enc: Option<SiguledXcoder>,

    /// The decoder
    pub dec: Option<XcoderType>,

    /// The value size shape propagated to matched fields; see [`SizeSpec`]
    pub size: Option<SizeSpec>,

    /// When a matched field's frame is left out of the encode; see [`SkipIf`]
    pub skip_if: Option<SkipIf>,

    /// syn errors
    pub errors: Option<syn::Error>,

    /// Whether an error on an entry of this type is reported, so a matched field reports no
    /// missing encoder or decoder of its own
    pub reported: bool,
}
/// [`DefaultXcoder`] implementation
impl DefaultXcoder {
    /// Adds a later entry of the same type: each setting it gives that this one left unset is
    /// taken, and one both give is a duplicate error at the later one
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the duplicates
    /// * `new` - The later entry
    pub(super) fn merge(&mut self, cx: &Ctxt, new: Self) {
        let typ = &new.typ;
        match (&self.dec, new.dec) {
            (Some(_), Some(dec)) => cx.error_spanned_by(dec, err!(DuplicateDefault(typ; "`dec`"))),
            (None, dec) => self.dec = dec,
            (Some(_), None) => (),
        }
        match (&self.enc, new.enc) {
            (Some(_), Some(enc)) => cx.error_spanned_by(enc, err!(DuplicateDefault(typ; "`enc`"))),
            (None, enc) => self.enc = enc,
            (Some(_), None) => (),
        }
        match (&self.size, new.size) {
            (Some(_), Some(size)) => cx.syn_error(syn::Error::new(
                size.span(),
                err!(DuplicateDefault(typ; "`size(..)`")),
            )),
            (None, size) => self.size = size,
            (Some(_), None) => (),
        }
        match (&self.skip_if, new.skip_if) {
            (Some(_), Some(skip_if)) => cx.syn_error(syn::Error::new(
                skip_if.span(),
                err!(DuplicateDefault(typ; "`skip_if_default` / `skip_if`")),
            )),
            (None, skip_if) => self.skip_if = skip_if,
            (Some(_), None) => (),
        }
        self.reported |= new.reported;
    }

    /// Checks the merged entries of a type: one of them sets an `enc` or a `dec`, and a
    /// `lengthed` size has the `dec` it describes. An entry that failed to parse holds only the
    /// settings before its error, so a type with an error reported is not checked
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the errors
    /// * `typ` - The type, as its first entry writes it, which an error points at
    pub(super) fn check(&mut self, cx: &Ctxt, typ: &syn::Type) {
        if self.reported {
            return;
        }
        if self.enc.is_none() && self.dec.is_none() {
            cx.error_spanned_by(typ, err!(MissingEncDecInDefault(typ)));
            self.reported = true;
        }
        if self.dec.is_none()
            && let Some(lengthed) = self.size.as_ref().and_then(SizeSpec::lengthed)
        {
            cx.syn_error(syn::Error::new(
                lengthed,
                err!(SizeLengthedWithoutDecInDefault),
            ));
            self.reported = true;
        }
    }
}
/// [`DefaultXcoder`] implementation of [`From`] for [`syn::MetaList`]
///
/// Parses the nested meta arguments from a `default(typ = .., enc = .., dec = .., size(..), ..)`
/// attribute list into a [`DefaultXcoder`]
///
/// Errors (a malformed or duplicate setting, a missing type, both skip forms) are recorded in
/// `DefaultXcoder::errors` rather than returned directly, so that the caller can accumulate
/// multiple diagnostics before aborting. What the entries of one type must provide together is
/// checked once they are merged ([`DefaultXcoder::check`])
impl From<&syn::MetaList> for DefaultXcoder {
    fn from(input: &syn::MetaList) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut typ: Option<TypeType> = None;
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut size: Option<SizeSpec> = None;
        let mut skip_if_default: Option<proc_macro2::Span> = None;
        let mut skip_if: Option<syn::Path> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let mut errors = input.parse_nested_meta(|meta| {
            let maybe_typ = typ
                .clone()
                .map_or(String::from("not defined"), |typ| typ.to_token_stream().to_string());
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownDefaultField(meta.path));
                6;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInDefault(maybe_typ)),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInDefault(maybe_typ)),
                typ: symbol::pnm_parse_maybestr_type    => err!(DuplicateTypeInDefault),
                size: symbol::parse_pnm_size            => err!(DuplicateSizeInDefault),
                skip_if_default: symbol::parse_pnm_skip_if_default => err!(SkipIfConflict),
                skip_if: symbol::pnm_parse_maybestr_skip_if => err!(SkipIfConflict),
            }
        }).err();
        // --------------------------------------------------
        // type must be provided; a list that failed to parse stopped at its error, so a type
        // written after it is not missing
        // --------------------------------------------------
        if typ.is_none() && errors.is_none() {
            errors = Some(syn::Error::new_spanned(
                input.clone(),
                err!(MissingTypeInDefault),
            ));
        }
        // --------------------------------------------------
        // the two skip forms contradict
        // --------------------------------------------------
        let skip_if = match SkipIf::from_items(skip_if_default, skip_if) {
            Ok(skip_if) => skip_if,
            Err(e) => {
                match errors {
                    None => errors = Some(e),
                    Some(ref mut errors) => errors.combine(e),
                }
                None
            }
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        DefaultXcoder {
            typ,
            enc,
            dec,
            size,
            skip_if,
            errors,
            reported: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Ctxt, Error,
        ast::attr::{MainContainer, size::SizeSpec},
    };

    #[test]
    /// Verifies the `default(typ = ..)` entries of one type are merged before they are checked
    ///
    /// The second entry sets only a size and the third only a skip. Neither has an `enc` or a
    /// `dec`, which the first gives the type, so the struct parses and its field takes all three
    fn entries_of_a_type_merge_before_the_check() {
        let item: syn::DeriveInput = syn::parse_quote! {
            #[klv(
                key(dec = decb::u8, enc = encb::u8),
                len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
                default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
                default(typ = u16, size(exact = 2)),
                default(typ = u16, skip_if_default),
            )]
            struct Reading {
                #[klv(key = 0x01)]
                level: u16,
            }
        };
        let cx = Ctxt::new();
        let container = MainContainer::from_ast(&cx, &item);
        cx.check().expect("the entries merge");
        let container = container.expect("the input is a container");
        let key = container.klv_keys().next().expect("one key");
        assert!(key.codec.enc.is_some() && key.codec.dec.is_some());
        assert_eq!(
            key.codec.size.as_ref().and_then(SizeSpec::exact_width),
            Some(2)
        );
        assert!(key.attrs.skip_if.is_some());
    }

    #[test]
    /// Verifies an entry with a mistake keeps the settings parsed before it
    ///
    /// The entry's `enc` and `dec` come before an unknown setting, so the field of its type has
    /// both, and the unknown setting is the one error
    fn an_entry_with_a_mistake_keeps_its_parsed_settings() {
        let item: syn::DeriveInput = syn::parse_quote! {
            #[klv(
                key(dec = decb::u8, enc = encb::u8),
                len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
                default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16, bogus = 1),
            )]
            struct Reading {
                #[klv(key = 0x01)]
                level: u16,
            }
        };
        let cx = Ctxt::new();
        let _ = MainContainer::from_ast(&cx, &item);
        let Err(errors) = cx.check() else {
            panic!("the unknown setting is an error");
        };
        let messages: Vec<String> = errors.into_iter().map(|err| err.to_string()).collect();
        assert_eq!(
            messages,
            [Error::UnknownDefaultField("bogus".to_owned()).to_string()]
        );
    }
}
