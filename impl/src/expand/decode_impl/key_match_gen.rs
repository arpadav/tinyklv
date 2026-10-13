//! Code generation for the key-dispatch match arms
//!
//! [`LoopCtx::items_match`] emits one arm per key of the `match` on the frame's key
//! ([`DecodeFraming::key`](super::framing::DecodeFraming::key)), routing the key to its value's
//! decoder and writing the result into the partial-packet accumulator. Each key reads its value
//! from the [`ValueSrc`] the caller picks for it. A sliced arm of a `size(exact = N)` field first
//! rejects a frame length other than `N`; [`LoopCtx::exact_guards`] runs the same checks on a frame
//! that runs past the end of the input, before it is classed as truncation. A
//! `size(implicit, exact = N)` arm cuts its own `N` bytes. A field's `break_on` decides what
//! happens to each value it decodes ([`LoopCtx::field_break`]). [`LoopCtx::unknown_arm`] and
//! [`LoopCtx::kept_sliced_unknown_arm`] emit the `_` arm that hands a key no field declares to
//! the `unknown(..)` decoder, reading from the main stream or from a `Sliced` frame's bounded
//! value
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    LoopCtx,
    framing::{DecodeFraming, UnknownDecoder},
    loop_driver::{ParseSite, TakeInto},
};
use crate::{
    ast::{
        attr::{
            FieldKey, UnknownField,
            size::{LenSource, SizeSpec},
        },
        symbol, types,
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{quote, quote_spanned};

/// Where an arm reads its value bytes from
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueSrc {
    /// `&mut subinput`, the bounded slice the frame body already cut using the header's length,
    /// which a `size(exact = N)` field first checks is `N` long; a known field's decode error is
    /// swallowed via `.ok()`
    HeaderSliced,
    /// `input` itself, because the value delimits itself; a decode error goes to
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail)
    Inline,
    /// `&mut subinput`, the `N` bytes the arm itself cuts off `input` for a
    /// `size(implicit, exact = N)` value, whose frame has no length; a decode error is swallowed
    /// via `.ok()`
    Exact(usize),
}

/// [`ValueSrc`] implementation
impl ValueSrc {
    /// The source of a `size(implicit)` value, which its arm reads with no length read before it
    ///
    /// # Arguments
    ///
    /// * `size` - The field's `size(..)`
    ///
    /// # Returns
    ///
    /// [`Self::Exact`] under `exact = N`, else [`Self::Inline`], or `None` when the value has a
    /// length prefix
    pub(super) fn implicit(size: Option<&SizeSpec>) -> Option<Self> {
        let size = size.filter(|size| size.implicit().is_some())?;
        Some(size.exact_width().map_or(Self::Inline, Self::Exact))
    }

    /// The source of a value with a length prefix, which its arm reads from the frame body's one
    /// `take`
    ///
    /// # Arguments
    ///
    /// * `size` - The field's `size(..)`
    ///
    /// # Returns
    ///
    /// [`Self::HeaderSliced`], or `None` for a `size(implicit)` value ([`Self::implicit`])
    pub(super) fn header_sliced(size: Option<&SizeSpec>) -> Option<Self> {
        Self::implicit(size).is_none().then_some(Self::HeaderSliced)
    }
}

/// [`LoopCtx`] implementation of the known-key arms
impl LoopCtx<'_> {
    /// Generates one arm per key of the `match` on the frame's key that `src_of` gives a source,
    /// each reading from that source: a field's own key, or each arm of its `key = oneof(..)`,
    /// whose value is wrapped in the arm's variant before the latebind
    ///
    /// `__acc` is the in-flight `XxxPartialPacket` accumulator; each arm writes `Some(value)` into
    /// its field. A duplicate key is LAST-WINS: a later successful decode overwrites the earlier
    /// value, of any arm of a `oneof(..)`. A failed sliced decode keeps the earlier value through
    /// `.ok() .. .or(__acc.#name)`; a failed [`ValueSrc::Inline`] decode leaves the iteration via
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail) and never
    /// writes, and a `lengthed` field's [`ValueSrc::Inline`] arm rejects without decoding. The
    /// caller splices the arms into its own `match`, ahead of its `_` arm
    ///
    /// # Arguments
    ///
    /// * `src_of` - Picks each key's value source from its `size(..)`, or `None` to leave the
    ///   key out of this `match`
    ///
    /// # Returns
    ///
    /// The concatenated match arms
    ///
    /// # Panics
    ///
    /// When a key has no decoder. [`DecodeCtx::new`](super::DecodeCtx::new) returns before arm
    /// generation unless every key has one
    pub(super) fn items_match(
        self,
        mut src_of: impl FnMut(Option<&SizeSpec>) -> Option<ValueSrc>,
    ) -> TokenStream {
        let arms = self.input.klv_keys().filter_map(|key| {
            let FieldKey {
                field,
                attrs,
                key,
                codec,
                ty,
                variant,
            } = key;
            let src = src_of(codec.size.as_ref())?;
            // --------------------------------------------------
            // decoder - the explicit codec, or the value type's `DecodeValue` impl
            // --------------------------------------------------
            let dec = if codec.fallback_dec {
                let stream = self.stream;
                quote! { <#ty as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
            } else {
                #[allow(
                    clippy::unwrap_used,
                    reason = "`DecodeCtx::new` returns before arm generation unless every key has a decoder"
                )]
                let dec = codec.dec.as_ref().unwrap();
                quote! { #dec }
            };
            // --------------------------------------------------
            // a `oneof(..)` arm's variant, then the latebind - optional transform post-decode,
            // mutating or consuming
            // --------------------------------------------------
            let wrap = variant.map(|variant| quote! { .map(#variant) });
            let latebind = match attrs.latebind.as_ref() {
                Some(lb) if lb.is_mut => {
                    let inner = &lb.inner;
                    quote! { #wrap .map(|mut __v| { #inner(&mut __v); __v }) }
                }
                Some(lb) => {
                    let inner = &lb.inner;
                    quote! { #wrap .map(#inner) }
                }
                None => quote! { #wrap },
            };
            let arm = ArmParts {
                ctx: self,
                name: &field.name,
                key,
                dec,
                size: codec.size.as_ref(),
                latebind,
                break_on: attrs.break_on.as_ref(),
                src,
            };
            Some(arm.emit())
        });
        quote! { #(#arms)* }
    }
}

/// [`LoopCtx`] implementation of the unknown-key arms
impl LoopCtx<'_> {
    /// Emits the `_` arm of a `match` on the frame's key, where the `unknown(..)` decoder reads
    /// the rest of a frame whose key no field declares from the main stream: every byte after the
    /// header, a key + len frame's length included
    ///
    /// ```text
    /// _ => match #decoder(#key)(input) {
    ///     Ok(__val) => #push,     // `Ok(_) => ()` when no field keeps the frame
    ///     Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    /// },
    /// // Separate
    /// _ => {
    ///     match #decoder(#key)(input) { .. }
    ///     #progress_check
    /// }
    /// ```
    ///
    /// The decoder is called with the key the way a `lengthed` decoder is called with the length
    /// ([`Self::unknown_call`]), on the cursor
    /// [`LoopDriver::read_input`](super::loop_driver::LoopDriver::read_input) hands it. Without a
    /// field to keep it the value is still decoded, which consumes it, and dropped. A failed
    /// decode is truncation or malformed as a [`ValueSrc::Inline`] arm's is
    /// ([`Self::unknown_match`]). A separate frame ends with this arm, which then rejects a frame
    /// whose key and value consumed no input ([`Self::progress_check`]); a fused one is checked
    /// after its value ([`Self::body`])
    ///
    /// # Arguments
    ///
    /// * `unknown` - The `unknown(dec = ..)` decoder and the field that keeps what it decodes
    ///
    /// # Returns
    ///
    /// The single match arm
    pub(super) fn unknown_arm(self, unknown: UnknownDecoder) -> TokenStream {
        let UnknownDecoder { decoder, keep } = unknown;
        let decoded = self
            .driver
            .read_input(|cursor| self.unknown_call(decoder, cursor));
        let decode = self.unknown_match(decoder, &decoded, keep, ValueSrc::Inline);
        // --------------------------------------------------
        // a separate frame ends with this arm, so it checks its own progress; a fused frame's
        // body checks after its framing `match`
        // --------------------------------------------------
        match self.framing {
            DecodeFraming::Separate { .. } => {
                let progress = self.progress_check();
                quote! {
                    _ => {
                        #decode
                        #progress
                    }
                }
            }
            DecodeFraming::Fused { .. } => quote! { _ => #decode, },
        }
    }

    /// Emits the `_` arm of a `Sliced` frame's `match` on its key, where the `unknown(..)` decoder
    /// reads the bounded value of a frame whose key no field declares, for the field that keeps it
    ///
    /// ```text
    /// _ => match #decoder(#key)(&mut subinput) {
    ///     Ok(__val) => #push,
    ///     Err(__err) => { let __err: ContextError = __err; #reject_value }
    /// },
    /// ```
    ///
    /// The decoder is called with the key ([`Self::unknown_call`]) on the bounded `subinput`,
    /// where [`Self::unknown_arm`] hands it the main stream. A failed decode is a malformed
    /// value labelled with the field, and never truncation ([`Self::unknown_match`])
    ///
    /// # Arguments
    ///
    /// * `decoder` - The `unknown(dec = ..)` decoder
    /// * `field` - The `#[klv(unknown)]` field the decoded frame is pushed into
    ///
    /// # Returns
    ///
    /// The single match arm
    pub(super) fn kept_sliced_unknown_arm(
        self,
        decoder: &types::XcoderType,
        field: &UnknownField,
    ) -> TokenStream {
        let decoded = self.unknown_call(decoder, &quote! { &mut subinput });
        let decode = self.unknown_match(decoder, &decoded, Some(field), ValueSrc::HeaderSliced);
        quote! { _ => #decode, }
    }

    /// Emits the `match` on the result of the `unknown(..)` decoder
    ///
    /// ```text
    /// match #decoded {
    ///     Ok(__val) => #push,     // `Ok(_) => ()` when no field keeps the frame
    ///     #value_err_arm
    /// }
    /// ```
    ///
    /// A decoded value is pushed into the field that keeps it, else dropped. A failed decode is
    /// rejected as a value read from `src` ([`Self::value_err_arm`]), labelled with the field,
    /// else with the struct's frame
    ///
    /// # Arguments
    ///
    /// * `decoder` - The `unknown(dec = ..)` decoder
    /// * `decoded` - Its call ([`Self::unknown_call`])
    /// * `keep` - The `#[klv(unknown)]` field, or `None` when no field keeps the frame
    /// * `src` - What the decoder read: the main stream, or a `Sliced` frame's bounded value
    ///
    /// # Returns
    ///
    /// The `match`
    fn unknown_match(
        self,
        decoder: &types::XcoderType,
        decoded: &TokenStream,
        keep: Option<&UnknownField>,
        src: ValueSrc,
    ) -> TokenStream {
        let on_ok = if let Some(field) = keep {
            let push = self.unknown_push(field);
            quote! { ::core::result::Result::Ok(__val) => #push, }
        } else {
            quote! { ::core::result::Result::Ok(_) => (), }
        };
        let on_err = self.value_err_arm(
            src,
            helpers::call_site_at(decoder),
            keep.map(|UnknownField { name, .. }| name),
            &format!("`{}` decoder", symbol::UNKNOWN_FRAMES),
        );
        quote! {
            match #decoded {
                #on_ok
                #on_err
            }
        }
    }

    /// Emits the `Err` arm of a `match` on a value decoder's result
    ///
    /// ```text
    /// Err(__err) => {
    ///     let __err: ContextError = __err;
    ///     #reject
    /// }
    /// ```
    ///
    /// The value is rejected with a [`Self::label`] naming `field`, else the struct's frame, and
    /// `decoder` in its reason. One read from the main stream ([`ValueSrc::Inline`]) is
    /// truncation at the end of the input, else malformed
    /// ([`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail)), `(its
    /// <decoder> failed before the end of the input)`. One read from a `Sliced` frame's bounded
    /// value is malformed
    /// ([`LoopDriver::reject_value`](super::loop_driver::LoopDriver::reject_value)), `(its
    /// <decoder> rejected the frame's value)`, and never truncation: the `take` already holds
    /// every byte of the frame. The error binding is located at the decoder, so a decoder of
    /// another error type is reported there
    ///
    /// # Arguments
    ///
    /// * `src` - What the decoder read
    /// * `at_dec` - The decoder's location
    /// * `field` - The field the value is for, `None` for the struct's frame
    /// * `decoder` - How the reason names the decoder
    ///
    /// # Returns
    ///
    /// The single match arm, every path leaving the iteration
    fn value_err_arm(
        self,
        src: ValueSrc,
        at_dec: proc_macro2::Span,
        field: Option<&syn::Ident>,
        decoder: &str,
    ) -> TokenStream {
        let reject = match src {
            ValueSrc::Inline => {
                let reason = format!("its {decoder} failed before the end of the input");
                let label = self.label(field, &reason);
                self.driver
                    .on_parse_fail(ParseSite::Value { label: &label })
            }
            ValueSrc::HeaderSliced | ValueSrc::Exact(_) => {
                let reason = format!("its {decoder} rejected the frame's value");
                self.driver.reject_value(&self.label(field, &reason))
            }
        };
        let err_ty = quote_spanned! {at_dec=>
            let __err: ::tinyklv::__export::winnow::error::ContextError = __err;
        };
        quote! {
            ::core::result::Result::Err(__err) => {
                #err_ty
                #reject
            }
        }
    }

    /// Emits the `unknown(..)` decoder called with the frame's key, reading `stream`
    ///
    /// The call, the key included, is located at the decoder, so a decoder of another signature
    /// (one that takes no key, or a key of another type) is reported there
    ///
    /// # Arguments
    ///
    /// * `decoder` - The `unknown(dec = ..)` decoder
    /// * `stream` - The cursor it reads: the main stream's, or `&mut subinput`
    ///
    /// # Returns
    ///
    /// `#decoder(#key)(#stream)`
    fn unknown_call(self, decoder: &types::XcoderType, stream: &TokenStream) -> TokenStream {
        let at_dec = helpers::call_site_at(decoder);
        let mut key = self.framing.key();
        key.set_span(at_dec);
        quote_spanned! {at_dec=> #decoder(#key)(#stream) }
    }

    /// Emits the push of a decoded unknown frame, `__val`, with its key into the field that keeps
    /// it, so a frame is kept only once its decoder returned `Ok`. A decoder that returns `Ok`
    /// without reading all of a `Sliced` frame's bounded value keeps what it returned, and the rest
    /// of the value goes with the frame's `take`
    ///
    /// The key, passed by value to the decoder and to the push, must be `Copy`. The push, the key
    /// included, is located at the field, so a field type with no `push`, or whose element is not
    /// the frame's `(key, value)`, is reported there
    ///
    /// # Arguments
    ///
    /// * `field` - The `#[klv(unknown)]` field
    ///
    /// # Returns
    ///
    /// `__acc.#name.push((#key, __val))`
    fn unknown_push(self, field: &UnknownField) -> TokenStream {
        let UnknownField { name, .. } = field;
        let at_field = helpers::call_site_at(name);
        let mut key = self.framing.key();
        key.set_span(at_field);
        quote_spanned! {at_field=> __acc.#name.push((#key, __val)) }
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

    /// Emits the check of a field's `break_on` on the value it just decoded, bound as `__val`
    ///
    /// ```text
    /// match #break_on(&__val) {
    ///     Skip => (),
    ///     Done => {
    ///         #write
    ///         break;
    ///     }
    ///     Abort(__msg) => {
    ///         #reject
    ///     }
    ///     _ => {
    ///         #write
    ///     }
    /// }
    /// ```
    ///
    /// `Skip` discards the value, `Done` keeps it and ends the loop, whose tail finalizes the
    /// accumulator, `Abort` rejects naming the field ([`LoopDriver::reject_aborted_value`]), and
    /// `Proceed`, like any later variant, keeps it
    ///
    /// # Arguments
    ///
    /// * `name` - The field
    /// * `break_on` - Its `fn(&value) -> BreakType`
    /// * `write` - The statement storing `__val` in the accumulator
    ///
    /// # Returns
    ///
    /// The `match`
    ///
    /// [`LoopDriver::reject_aborted_value`]: super::loop_driver::LoopDriver::reject_aborted_value
    fn field_break(
        self,
        name: &syn::Ident,
        break_on: &syn::Path,
        write: &TokenStream,
    ) -> TokenStream {
        let reason = format!("its `{}` returned `Abort`", symbol::BREAK_ON);
        let reject = self
            .driver
            .reject_aborted_value(&self.label(Some(name), &reason));
        quote! {
            match #break_on(&__val) {
                ::tinyklv::BreakType::Skip => (),
                ::tinyklv::BreakType::Done => {
                    #write
                    break;
                }
                ::tinyklv::BreakType::Abort(__msg) => {
                    #reject
                }
                // `Proceed`, and any later `BreakType` variant, keeps the value
                _ => {
                    #write
                }
            }
        }
    }

    /// Emits the checks a frame running past the end of the input meets before it is classed as
    /// truncation: one per distinct `size(exact = N)` width of a key with a length prefix
    /// ([`SizeSpec::len_check`]), in field order, rejecting a key of that width with a length
    /// other than `N`. A width that fields share tells the rejected field apart only once its
    /// check fails. Each check carries the `clippy::manual_range_patterns` allow its pattern
    /// needs, if any ([`super::key_pattern`])
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
        // the exact keys, (key, name, `N` as written), grouped by width in field order
        // --------------------------------------------------
        let mut groups: Vec<(usize, Vec<_>)> = Vec::new();
        for FieldKey {
            field, key, codec, ..
        } in self.input.klv_keys()
        {
            let Some((width, lit)) = codec.size.as_ref().and_then(SizeSpec::len_check) else {
                continue;
            };
            let field = (key, &field.name, lit);
            match groups.iter_mut().find(|(w, _)| *w == width) {
                Some((_, fields)) => fields.push(field),
                None => groups.push((width, vec![field])),
            }
        }
        // --------------------------------------------------
        // one check per width, then one test per key where fields share the width
        // --------------------------------------------------
        let guards = groups.iter().map(|(width, fields)| {
            let keys: Vec<_> = fields.iter().map(|(key, ..)| *key).collect();
            let (pattern, allow) = super::key_pattern(&keys);
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
                #allow
                if ::core::matches!(#key, #pattern) && #len != #width {
                    #reject
                }
            }
        });
        quote! { #(#guards)* }
    }
}

/// The per-key fragments of one known-key arm
struct ArmParts<'a> {
    /// The loop context the arm is emitted against
    ctx: LoopCtx<'a>,

    /// The field the arm writes into, `__acc.#name`
    name: &'a syn::Ident,

    /// The key literal, the arm's pattern: the field's `#[klv(key = ..)]`, or a `oneof(..)` arm's
    key: &'a syn::Lit,

    /// The decoder expression (explicit codec or `DecodeValue` fallback)
    dec: TokenStream,

    /// The value's `size(..)`: whether the decoder takes a length, and the width a frame's length
    /// must match
    size: Option<&'a SizeSpec>,

    /// The `.map(..)` of a `oneof(..)` arm's variant, then the latebind transform, else empty
    latebind: TokenStream,

    /// The field's `fn(&value) -> BreakType`, which decides what happens to a decoded value
    break_on: Option<&'a syn::Path>,

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
    /// // Exact(N)
    /// #key => {
    ///     #take(N)
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
    /// // HeaderSliced or Exact(N), `break_on`
    /// #key => {
    ///     #prelude
    ///     if let Ok(__val) = #dec #len_arg (&mut subinput) #latebind { #field_break }
    /// },
    /// // Inline, `break_on`: as Inline, matching `#val #latebind` and the `Ok` arm `#field_break`
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
            ValueSrc::HeaderSliced => {
                let check = self.exact_check();
                self.sliced(check)
            }
            // --------------------------------------------------
            // exact: decode the `N` bytes the arm takes, a short frame being truncation
            // --------------------------------------------------
            ValueSrc::Exact(width) => {
                let take = self.ctx.take(TakeInto::Exact(width));
                self.sliced(Some(take))
            }
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
        let (width, lit) = self.size.and_then(SizeSpec::len_check)?;
        let ctx = self.ctx;
        let len = ctx.framing.len();
        let reject = ctx.exact_reject(self.name, lit);
        Some(quote! {
            if #len != #width {
                #reject
            }
        })
    }

    /// Emits a sliced arm, which decodes `subinput` after `prelude`: the frame's length check
    /// ([`Self::exact_check`]), or the `take` that cuts an `implicit` value's `N` bytes
    ///
    /// `#len_arg` is the length a `lengthed` decoder is passed ([`SizeSpec::len_source`]): the
    /// frame's length, or `N` under `size(exact = N)`, which the arm's check made the frame's
    /// length or its `take` cut. The value expression and `#len_arg` are located at the decoder,
    /// so a decoder of another value type, or one that takes no length under `size(lengthed)`, is
    /// reported there
    ///
    /// # Arguments
    ///
    /// * `prelude` - What runs before the decode, if anything
    ///
    /// # Returns
    ///
    /// The single match arm
    fn sliced(self, prelude: Option<TokenStream>) -> TokenStream {
        let log = self.log();
        let Self {
            ctx,
            name,
            key,
            dec,
            size,
            latebind,
            break_on,
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
        // the decode result, bound and logged first under `debug`
        // --------------------------------------------------
        let (bind, decoded) = match log {
            None => (
                None,
                quote_spanned! {at_dec=> #dec #len_arg (&mut subinput) },
            ),
            Some(log) => {
                let val = ctx.framing.val();
                let mut val_at_dec = val.clone();
                val_at_dec.set_span(at_dec);
                let bind = quote! {
                    let #val = #dec #len_arg (&mut subinput);
                    #log
                };
                (Some(bind), quote! { #val_at_dec })
            }
        };
        // --------------------------------------------------
        // a decoded value goes where the field's `break_on` decides
        // --------------------------------------------------
        if let Some(break_on) = break_on {
            let write = quote_spanned! {at_dec=>
                __acc.#name = ::core::option::Option::Some(__val);
            };
            let check = ctx.field_break(name, break_on, &write);
            let mapped = quote_spanned! {at_dec=> #decoded #latebind };
            return quote! {
                #key => {
                    #prelude
                    #bind
                    if let ::core::result::Result::Ok(__val) = #mapped {
                        #check
                    }
                },
            };
        }
        let value = quote_spanned! {at_dec=> #decoded.ok() #latebind .or(__acc.#name) };
        // --------------------------------------------------
        // a bare write when nothing runs before it, else a block
        // --------------------------------------------------
        match (prelude, bind) {
            (None, None) => quote! {
                #key => __acc.#name = #value,
            },
            (prelude, bind) => quote! {
                #key => {
                    #prelude
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
            break_on,
            ..
        } = self;
        let at_dec = helpers::call_site_at(&dec);
        let val = ctx.framing.val();
        let on_err = ctx.value_err_arm(ValueSrc::Inline, at_dec, Some(name), "decoder");
        let some_val = quote_spanned! {at_dec=> ::core::option::Option::Some(__val) };
        // --------------------------------------------------
        // the decoded value is written, or goes where the field's `break_on` decides once the
        // latebind has mapped it
        // --------------------------------------------------
        let (mapped, on_ok) = match break_on {
            None => (None, quote! { __acc.#name = #some_val #latebind; }),
            Some(break_on) => {
                let write = quote! { __acc.#name = #some_val; };
                (Some(latebind), ctx.field_break(name, break_on, &write))
            }
        };
        quote! {
            #key => {
                let #val = #dec (input);
                #log
                match #val #mapped {
                    ::core::result::Result::Ok(__val) => {
                        #on_ok
                    }
                    #on_err
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
