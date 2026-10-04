//! Code generation for the key-dispatch match arms
//!
//! [`ArmCtx::items_match`] emits one `match key` arm per field, routing the key to the field's
//! decoder and writing the result into the partial-packet accumulator. Each field reads its value
//! from the [`ValueSrc`] the caller picks for it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    helpers,
    loop_driver::{LoopCtx, ParseSite},
};
use crate::ast::attr::{MainField, size::SizeSpec};

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

/// The per-loop context every known-key arm is emitted against
///
/// One is built per emitted loop, because its [`LoopCtx`] carries that loop's driver
#[derive(Clone, Copy)]
pub(super) struct ArmCtx<'a> {
    /// The struct being decoded, named in malformed-value labels
    pub(super) container: &'a syn::Ident,

    /// All fields of the container; those without KLV annotations get no arm
    pub(super) fields: &'a [MainField<'a>],

    /// Whether each arm logs the field name and decode result
    pub(super) debug: bool,

    /// The driver, stream type and accumulator type of the loop the arms are emitted into
    pub(super) lp: LoopCtx<'a>,
}

/// [`ArmCtx`] implementation
impl ArmCtx<'_> {
    /// Generates one `match key` arm per KLV field, each reading from the source `src_of` picks
    ///
    /// `__acc` is the in-flight `XxxPartialPacket` accumulator; each arm writes `Some(value)` into
    /// its field. A duplicate key is LAST-WINS: a later successful decode overwrites the earlier
    /// value. A failed [`ValueSrc::HeaderSliced`] decode keeps the earlier value through
    /// `.ok() .. .or(__acc.#name)`; a failed [`ValueSrc::Inline`] decode leaves the iteration via
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail) and never
    /// writes. The caller splices the arms into its own
    /// `match key { #arms _ => .. }`
    ///
    /// # Arguments
    ///
    /// * `src_of` - Picks each field's value source
    ///
    /// # Returns
    ///
    /// The concatenated match arms
    pub(super) fn items_match(&self, src_of: impl Fn(&MainField) -> ValueSrc) -> TokenStream {
        let arms = self.fields.iter().filter_map(|f| {
            let attrs = f.attrs.as_ref()?;
            // --------------------------------------------------
            // decoder - the explicit codec, or the field type's `DecodeValue` impl
            // --------------------------------------------------
            let dec = if attrs.fallback_dec {
                let t = helpers::unwrap_option_type(f.ty).unwrap_or(f.ty);
                let stream = self.lp.stream;
                quote! { <#t as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
            } else {
                #[allow(
                    clippy::unwrap_used,
                    reason = "`gen_decode_impl` call ensures that `attrs.dec` is `Some`"
                )]
                let dec = attrs.dec.as_ref().unwrap();
                quote! { #dec }
            };
            // --------------------------------------------------
            // size - a `var` size makes the decoder take the runtime `len`; a reserve-only size or
            // an omitted one reads without it
            // --------------------------------------------------
            let len_arg = if attrs.size.is_some_and(SizeSpec::takes_len) {
                quote! { (len) }
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
            Some(
                ArmParts {
                    ctx: self,
                    name: &f.name,
                    key: &attrs.key,
                    dec,
                    len_arg,
                    latebind,
                    src: src_of(f),
                }
                .emit(),
            )
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
    /// A `concat!(..)` token expression evaluating to a `&'static str` that names
    /// `` `<container>::<field>` ``
    fn malformed_value_message(&self, field: &syn::Ident) -> TokenStream {
        let container = self.container;
        quote! {
            concat!(
                "`",
                stringify!(#container),
                "::",
                stringify!(#field),
                "` value (its decoder failed before the end of the input)",
            )
        }
    }
}

/// The per-field fragments of one known-key arm
struct ArmParts<'a> {
    /// The container context the arm is emitted against
    ctx: &'a ArmCtx<'a>,

    /// The field the arm writes into, `__acc.#name`
    name: &'a syn::Ident,

    /// The field's `#[klv(key = ..)]` literal, the arm's pattern
    key: &'a syn::Lit,

    /// The decoder expression (explicit codec or `DecodeValue` fallback)
    dec: TokenStream,

    /// The `(len)` argument of a `var`-sized decoder, else empty
    len_arg: TokenStream,

    /// The `.map(..)` latebind transform, else empty
    latebind: TokenStream,

    /// Where the value bytes are read from
    src: ValueSrc,
}

/// [`ArmParts`] implementation
impl ArmParts<'_> {
    /// Emits the `match key` arm for this field
    ///
    /// ```text
    /// // HeaderSliced
    /// #key => __acc.#name = #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name),
    /// // Inline
    /// #key => {
    ///     let val = #dec #len_arg (input);
    ///     match val {
    ///         Ok(__val) => { __acc.#name = Some(__val) #latebind; }
    ///         Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    ///     }
    /// },
    /// ```
    ///
    /// With `debug` set, the decode result is logged before it is used (a `HeaderSliced` arm then
    /// binds it to `val` too). The `Inline` error binding is located at the decoder, so a decoder
    /// with another error type is reported there (the rule is stated on `tinyklv::Framing::Inline`)
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
        let log = ctx.debug.then(|| {
            let logger = crate::expand::logger::debug_logger();
            quote! { #logger ("\t{}: {:?}", stringify!(#name), val); }
        });
        match (src, log) {
            // --------------------------------------------------
            // header-sliced: read the bounded slice, swallow value errors
            // --------------------------------------------------
            (ValueSrc::HeaderSliced, None) => quote! {
                #key => __acc.#name = #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name),
            },
            (ValueSrc::HeaderSliced, Some(log)) => quote! {
                #key => {
                    let val = #dec #len_arg (&mut subinput);
                    #log
                    __acc.#name = val.ok() #latebind .or(__acc.#name);
                },
            },
            // --------------------------------------------------
            // inline: read the main stream; a failed decode is truncation or malformed
            // --------------------------------------------------
            (ValueSrc::Inline, log) => {
                let label = ctx.malformed_value_message(name);
                let driver = ctx.lp.driver;
                let on_fail = driver.on_parse_fail(ParseSite::Value { label: &label });
                let err_ty = quote_spanned! {helpers::call_site_at(&dec)=>
                    let __err: ::tinyklv::__export::winnow::error::ContextError = __err;
                };
                quote! {
                    #key => {
                        let val = #dec #len_arg (input);
                        #log
                        match val {
                            ::core::result::Result::Ok(__val) => {
                                __acc.#name = ::core::option::Option::Some(__val) #latebind;
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
