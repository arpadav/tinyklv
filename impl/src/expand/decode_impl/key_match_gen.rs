//! Code generation for the key-dispatch match arms
//!
//! [`LoopCtx::items_match`] emits one arm per field of the `match` on the frame's key
//! ([`DecodeFraming::key`](super::framing::DecodeFraming::key)), routing the key to the field's
//! decoder and writing the result into the partial-packet accumulator. Each field reads its value
//! from the [`ValueSrc`] the caller picks for it. A sliced arm of a `size(exact = N)` field first
//! rejects a frame length other than `N`; [`LoopCtx::exact_guards`] runs the same checks on a frame
//! that runs past the end of the input, before it is classed as truncation
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{LoopCtx, loop_driver::ParseSite};
use crate::{
    ast::{
        attr::{
            MainField,
            size::{LenSource, SizeSpec},
        },
        symbol,
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{quote, quote_spanned};

/// Where a known-key arm reads its value bytes from
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueSrc {
    /// `&mut subinput`, the bounded slice the frame body already cut using the header's length,
    /// which a `size(exact = N)` field first checks is `N` long; a decode error is swallowed via
    /// `.ok()`
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
    /// value. A failed sliced decode keeps the earlier value through `.ok() .. .or(__acc.#name)`; a
    /// failed [`ValueSrc::Inline`] decode leaves the iteration via
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail) and never
    /// writes, and a `lengthed` field's [`ValueSrc::Inline`] arm rejects without decoding. The
    /// caller splices the arms into its own `match`, ahead of its `_` arm
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
                size: attrs.size.as_ref(),
                latebind,
                src: src_of(f),
            }
            .emit()
        });
        quote! { #(#arms)* }
    }
}

/// [`LoopCtx`] implementation of the `size(exact = N)` length check
impl LoopCtx<'_> {
    /// Emits the rejection of a frame whose length is not its field's `size(exact = N)`, labelled
    /// with the field and `N` as the field wrote it
    /// ([`LoopDriver::reject_exact`](super::loop_driver::LoopDriver::reject_exact))
    ///
    /// # Arguments
    ///
    /// * `name` - The field
    /// * `lit` - Its `N`
    ///
    /// # Returns
    ///
    /// The statements, every path leaving the loop
    fn exact_reject(self, name: &syn::Ident, lit: &syn::LitInt) -> TokenStream {
        let reason = format!(
            "its frame length does not match `{}({} = {lit})`",
            symbol::SIZE,
            symbol::EXACT,
        );
        self.driver.reject_exact(&self.label(Some(name), &reason))
    }

    /// Emits the checks a frame running past the end of the input meets before it is classed as
    /// truncation: one per distinct `size(exact = N)` width, in field order, rejecting a key of
    /// that width with a length other than `N`. A width that fields share tells the rejected
    /// field apart only once its check fails
    ///
    /// ```text
    /// if matches!(#key, K1) && #len != N1 { #reject(K1) }
    /// if matches!(#key, K2 | K3) && #len != N2 {
    ///     if matches!(#key, K2) { #reject(K2) }
    ///     if matches!(#key, K3) { #reject(K3) }
    /// }
    /// ```
    ///
    /// [`LoopCtx::take`] rewinds to the frame start before the checks, so a rejection leaves the
    /// cursor there
    ///
    /// # Returns
    ///
    /// The checks, empty without a `size(exact = N)` field
    pub(super) fn exact_guards(self) -> TokenStream {
        let key = self.framing.key();
        let len = self.framing.len();
        // --------------------------------------------------
        // the exact fields, (key, name, `N` as written), grouped by width in field order
        // --------------------------------------------------
        let mut groups: Vec<(usize, Vec<_>)> = Vec::new();
        for (f, attrs) in self.input.klv_fields() {
            let Some((width, lit)) = attrs.size.as_ref().and_then(SizeSpec::exact) else {
                continue;
            };
            let field = (&attrs.key, &f.name, lit);
            match groups.iter_mut().find(|(w, _)| *w == width) {
                Some((_, fields)) => fields.push(field),
                None => groups.push((width, vec![field])),
            }
        }
        // --------------------------------------------------
        // one check per width, then one test per key where fields share the width
        // --------------------------------------------------
        let guards = groups.iter().map(|(width, fields)| {
            let pats = fields.iter().map(|(pat, ..)| pat);
            let reject = if let [(_, name, lit)] = fields.as_slice() {
                self.exact_reject(name, lit)
            } else {
                let rejects = fields.iter().map(|(pat, name, lit)| {
                    let reject = self.exact_reject(name, lit);
                    quote! { if ::core::matches!(#key, #pat) { #reject } }
                });
                quote! { #(#rejects)* }
            };
            quote! {
                if ::core::matches!(#key, #(#pats)|*) && #len != #width {
                    #reject
                }
            }
        });
        quote! { #(#guards)* }
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

    /// The field's `size(..)`: whether the decoder takes a length, and the width a frame's length
    /// must match
    size: Option<&'a SizeSpec>,

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
    /// // HeaderSliced, `size(exact = N)`
    /// #key => {
    ///     #exact_check
    ///     __acc.#name = #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name);
    /// },
    /// // Inline
    /// #key => {
    ///     let #val = #dec (input);
    ///     match #val {
    ///         Ok(__val) => { __acc.#name = Some(__val) #latebind; }
    ///         Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    ///     }
    /// },
    /// // Inline, `lengthed`
    /// #key => { #reject },
    /// ```
    ///
    /// `#val` is [`DecodeFraming::val`](super::framing::DecodeFraming::val). With `debug` set,
    /// the decode result is logged before it is used (a sliced arm then binds it too). The
    /// `Inline` error binding and `Some(__val)` are located at the decoder, so a decoder with
    /// another error type (the rule is stated on `tinyklv::Framing::Inline`) or value type is
    /// reported there. An `Inline` frame carries no length to pass a `lengthed` decoder, so its
    /// value is malformed, rejected with a label naming the field and the cursor still at the
    /// value start
    ///
    /// # Returns
    ///
    /// The single match arm
    fn emit(self) -> TokenStream {
        match self.src {
            // --------------------------------------------------
            // sliced: decode the bounded slice, once its length is checked
            // --------------------------------------------------
            ValueSrc::HeaderSliced => self.sliced(),
            // --------------------------------------------------
            // inline, lengthed: no length to pass the decoder, so the value is malformed
            // --------------------------------------------------
            ValueSrc::Inline if self.size.is_some_and(SizeSpec::takes_len) => {
                let Self { ctx, name, key, .. } = self;
                let reason = format!(
                    "an `Inline` frame has no length for its `{}({})` decoder",
                    symbol::SIZE,
                    symbol::LENGTHED,
                );
                let label = ctx.label(Some(name), &reason);
                let reject = ctx.driver.reject_label(&label);
                quote! {
                    #key => {
                        #reject
                    },
                }
            }
            // --------------------------------------------------
            // inline: read the main stream; a failed decode is truncation or malformed
            // --------------------------------------------------
            ValueSrc::Inline => self.inline(),
        }
    }

    /// Emits the length check that opens the sliced arm of a `size(exact = N)` field
    ///
    /// ```text
    /// if #len != N {
    ///     #reject
    /// }
    /// ```
    ///
    /// The rejection ([`LoopCtx::exact_reject`]) leaves the loop, its cursor past the frame
    ///
    /// # Returns
    ///
    /// The check, `None` without `size(exact = N)`
    fn exact_check(&self) -> Option<TokenStream> {
        let (width, lit) = self.size.and_then(SizeSpec::exact)?;
        let ctx = self.ctx;
        let len = ctx.framing.len();
        let reject = ctx.exact_reject(self.name, lit);
        Some(quote! {
            if #len != #width {
                #reject
            }
        })
    }

    /// Emits a sliced arm, which decodes `subinput` after its length check ([`Self::exact_check`])
    ///
    /// `#len_arg` is the length a `lengthed` decoder is passed ([`SizeSpec::len_source`]): the
    /// frame's length, or `N` under `size(exact = N)`, which the arm's check made the frame's
    /// length. The value expression and `#len_arg` are located at the decoder, so a decoder of
    /// another value type, or one that takes no length under `size(lengthed)`, is reported there
    ///
    /// # Returns
    ///
    /// The single match arm
    fn sliced(self) -> TokenStream {
        let check = self.exact_check();
        let log = self.log();
        let Self {
            ctx,
            name,
            key,
            dec,
            size,
            latebind,
            ..
        } = self;
        let at_dec = helpers::call_site_at(&dec);
        // --------------------------------------------------
        // the length a `lengthed` decoder is passed, located at the decoder
        // --------------------------------------------------
        let len_arg = size.and_then(SizeSpec::len_source).map(|source| {
            let mut len = match source {
                LenSource::Frame => TokenTree::from(ctx.framing.len()),
                LenSource::Exact(width) => TokenTree::from(Literal::usize_suffixed(width)),
            };
            len.set_span(at_dec);
            quote_spanned! {at_dec=> (#len) }
        });
        // --------------------------------------------------
        // the decode, bound and logged first under `debug`
        // --------------------------------------------------
        let (bind, value) = match log {
            None => {
                let value = quote_spanned! {at_dec=>
                    #dec #len_arg (&mut subinput).ok() #latebind .or(__acc.#name)
                };
                (None, value)
            }
            Some(log) => {
                let val = ctx.framing.val();
                let mut val_at_dec = val.clone();
                val_at_dec.set_span(at_dec);
                let bind = quote! {
                    let #val = #dec #len_arg (&mut subinput);
                    #log
                };
                let value = quote_spanned! {at_dec=>
                    #val_at_dec.ok() #latebind .or(__acc.#name)
                };
                (Some(bind), value)
            }
        };
        // --------------------------------------------------
        // a bare write when nothing runs before it, else a block
        // --------------------------------------------------
        match (check, bind) {
            (None, None) => quote! {
                #key => __acc.#name = #value,
            },
            (check, bind) => quote! {
                #key => {
                    #check
                    #bind
                    __acc.#name = #value;
                },
            },
        }
    }

    /// Emits an `Inline` arm, which decodes the main stream
    ///
    /// # Returns
    ///
    /// The single match arm
    fn inline(self) -> TokenStream {
        let log = self.log();
        let Self {
            ctx,
            name,
            key,
            dec,
            latebind,
            ..
        } = self;
        let at_dec = helpers::call_site_at(&dec);
        let val = ctx.framing.val();
        let label = ctx.label(Some(name), "its decoder failed before the end of the input");
        let on_fail = ctx.driver.on_parse_fail(ParseSite::Value { label: &label });
        let err_ty = quote_spanned! {at_dec=>
            let __err: ::tinyklv::__export::winnow::error::ContextError = __err;
        };
        let some_val = quote_spanned! {at_dec=> ::core::option::Option::Some(__val) };
        quote! {
            #key => {
                let #val = #dec (input);
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

    /// Emits the `debug` log of the decode result, when the container set `debug`
    ///
    /// # Returns
    ///
    /// The log statement, or `None` without `debug`
    fn log(&self) -> Option<TokenStream> {
        let Self { ctx, name, .. } = *self;
        ctx.input.attrs.debug.is_some().then(|| {
            let logger = crate::expand::logger::debug_logger();
            let val = ctx.framing.val();
            quote! { #logger ("\t{}: {:?}", stringify!(#name), #val); }
        })
    }
}
