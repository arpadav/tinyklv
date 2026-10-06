//! Code generation for the key-dispatch match arms
//!
//! [`LoopCtx::items_match`] emits one arm per field of the `match` on the frame's key
//! ([`DecodeFraming::key`](super::framing::DecodeFraming::key)), routing the key to the field's
//! decoder and writing the result into the partial-packet accumulator. Each field reads its value
//! from the [`ValueSrc`] the caller picks for it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{LoopCtx, loop_driver::ParseSite};
use crate::{
    ast::attr::{MainField, size::SizeSpec},
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

/// Where a known-key arm reads its value bytes from
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueSrc {
    /// `&mut subinput`, the bounded slice the frame body already cut using the header's length;
    /// a decode error is swallowed via `.ok()`
    HeaderSliced,
    /// `input` itself, because the value delimits itself; a decode error goes to
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail)
    Inline,
}

/// [`LoopCtx`] implementation of the known-key arms
impl LoopCtx<'_> {
    /// Generates one arm per KLV field of the `match` on the frame's key, each reading from the
    /// source `src_of` picks
    ///
    /// `__acc` is the in-flight `XxxPartialPacket` accumulator; each arm writes `Some(value)` into
    /// its field. A duplicate key is LAST-WINS: a later successful decode overwrites the earlier
    /// value. A failed [`ValueSrc::HeaderSliced`] decode keeps the earlier value through
    /// `.ok() .. .or(__acc.#name)`; a failed [`ValueSrc::Inline`] decode leaves the iteration via
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail) and never
    /// writes. The caller splices the arms into its own `match`, ahead of its `_` arm
    ///
    /// # Arguments
    ///
    /// * `src_of` - Picks each field's value source
    ///
    /// # Returns
    ///
    /// The concatenated match arms
    ///
    /// # Panics
    ///
    /// When a field has no decoder. [`DecodeCtx::new`](super::DecodeCtx::new) returns before arm
    /// generation unless every field has one
    pub(super) fn items_match(self, mut src_of: impl FnMut(&MainField) -> ValueSrc) -> TokenStream {
        let arms = self.input.klv_fields().map(|(f, attrs)| {
            // --------------------------------------------------
            // decoder - the explicit codec, or the field type's `DecodeValue` impl
            // --------------------------------------------------
            let dec = if attrs.fallback_dec {
                let t = f.value_ty();
                let stream = self.stream;
                quote! { <#t as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
            } else {
                #[allow(
                    clippy::unwrap_used,
                    reason = "`DecodeCtx::new` returns before arm generation unless every field has a decoder"
                )]
                let dec = attrs.dec.as_ref().unwrap();
                quote! { #dec }
            };
            // --------------------------------------------------
            // size - a `var` size makes the decoder take the frame's length; a reserve-only size or
            // an omitted one reads without it
            // --------------------------------------------------
            let len_arg = if attrs.size.is_some_and(SizeSpec::takes_len) {
                let len = self.framing.len();
                quote! { (#len) }
            } else {
                quote! {}
            };
            // --------------------------------------------------
            // latebind - optional transform post-decode, mutating or consuming
            // --------------------------------------------------
            let latebind = match attrs.latebind.as_ref() {
                Some(lb) if lb.is_mut => {
                    let inner = &lb.inner;
                    quote! { .map(|mut __v| { #inner(&mut __v); __v }) }
                }
                Some(lb) => {
                    let inner = &lb.inner;
                    quote! { .map(#inner) }
                }
                None => quote! {},
            };
            ArmParts {
                ctx: self,
                name: &f.name,
                key: &attrs.key,
                dec,
                len_arg,
                latebind,
                src: src_of(f),
            }
            .emit()
        });
        quote! { #(#arms)* }
    }

    /// Generates the label naming a field whose `Inline` value decoder failed before the end of
    /// the input
    ///
    /// A noun phrase, so that it reads after the `invalid ` that the streaming driver prefixes and
    /// that winnow's `ContextError` display prints before a one-shot label
    ///
    /// # Arguments
    ///
    /// * `field` - The field whose decoder failed
    ///
    /// # Returns
    ///
    /// A [`LoopDriver::reject_label`](super::loop_driver::LoopDriver::reject_label) label naming
    /// `` `<container>::<field>` ``
    fn malformed_value_label(self, field: &syn::Ident) -> TokenStream {
        let container = &self.input.ident;
        quote! {
            ::core::concat!(
                "`",
                ::core::stringify!(#container),
                "::",
                ::core::stringify!(#field),
                "` value (its decoder failed before the end of the input)",
            )
        }
    }
}

/// The per-field fragments of one known-key arm
struct ArmParts<'a> {
    /// The loop context the arm is emitted against
    ctx: LoopCtx<'a>,

    /// The field the arm writes into, `__acc.#name`
    name: &'a syn::Ident,

    /// The field's `#[klv(key = ..)]` literal, the arm's pattern
    key: &'a syn::Lit,

    /// The decoder expression (explicit codec or `DecodeValue` fallback)
    dec: TokenStream,

    /// The length argument of a `var`-sized decoder, else empty
    len_arg: TokenStream,

    /// The `.map(..)` latebind transform, else empty
    latebind: TokenStream,

    /// Where the value bytes are read from
    src: ValueSrc,
}

/// [`ArmParts`] implementation
impl ArmParts<'_> {
    /// Emits this field's arm of the `match` on the frame's key
    ///
    /// ```text
    /// // HeaderSliced
    /// #key => __acc.#name = #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name),
    /// // Inline
    /// #key => {
    ///     let #val = #dec #len_arg (input);
    ///     match #val {
    ///         Ok(__val) => { __acc.#name = Some(__val) #latebind; }
    ///         Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    ///     }
    /// },
    /// ```
    ///
    /// `#val` is [`DecodeFraming::val`](super::framing::DecodeFraming::val). With `debug` set,
    /// the decode result is logged before it is used (a `HeaderSliced` arm then binds it too). The
    /// `HeaderSliced` value expression, and the `Inline` error binding and `Some(__val)`, are
    /// located at the decoder, so a decoder with another error type (the rule is stated on
    /// `tinyklv::Framing::Inline`) or value type is reported there
    ///
    /// # Returns
    ///
    /// The single match arm
    fn emit(self) -> TokenStream {
        let Self {
            ctx,
            name,
            key,
            dec,
            len_arg,
            latebind,
            src,
        } = self;
        let at_dec = helpers::call_site_at(&dec);
        let val = ctx.framing.val();
        let log = ctx.input.attrs.debug.is_some().then(|| {
            let logger = crate::expand::logger::debug_logger();
            quote! { #logger ("\t{}: {:?}", stringify!(#name), #val); }
        });
        match (src, log) {
            // --------------------------------------------------
            // header-sliced: read the bounded slice, swallow value errors
            // --------------------------------------------------
            (ValueSrc::HeaderSliced, None) => {
                let value = quote_spanned! {at_dec=>
                    #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name)
                };
                quote! {
                    #key => __acc.#name = #value,
                }
            }
            (ValueSrc::HeaderSliced, Some(log)) => {
                let mut val_at_dec = val.clone();
                val_at_dec.set_span(at_dec);
                let value = quote_spanned! {at_dec=>
                    #val_at_dec.ok() #latebind .or(__acc.#name)
                };
                quote! {
                    #key => {
                        let #val = #dec #len_arg (&mut subinput);
                        #log
                        __acc.#name = #value;
                    },
                }
            }
            // --------------------------------------------------
            // inline: read the main stream; a failed decode is truncation or malformed
            // --------------------------------------------------
            (ValueSrc::Inline, log) => {
                let label = ctx.malformed_value_label(name);
                let on_fail = ctx.driver.on_parse_fail(ParseSite::Value { label: &label });
                let err_ty = quote_spanned! {at_dec=>
                    let __err: ::tinyklv::__export::winnow::error::ContextError = __err;
                };
                let some_val = quote_spanned! {at_dec=> ::core::option::Option::Some(__val) };
                quote! {
                    #key => {
                        let #val = #dec #len_arg (input);
                        #log
                        match #val {
                            ::core::result::Result::Ok(__val) => {
                                __acc.#name = #some_val #latebind;
                            }
                            ::core::result::Result::Err(__err) => {
                                #err_ty
                                #on_fail
                            }
                        }
                    },
                }
            }
        }
    }
}
