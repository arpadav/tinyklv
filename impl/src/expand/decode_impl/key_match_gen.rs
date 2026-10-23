//! Code generation for the key-dispatch match arms
//!
//! [`LoopCtx::items_match`] emits one arm per key of the `match` on the frame's key
//! ([`DecodeFraming::key`]), routing the key to its value's decoder and writing the result into
//! the partial-packet accumulator. Each key reads its value
//! from the [`ValueSrc`] the caller picks for it. A sliced arm of a `size(exact = N)` field first
//! rejects a frame length other than `N`; [`LoopCtx::exact_guards`] runs the same checks on a frame
//! that runs past the end of the input, before it is classed as truncation. A
//! `size(implicit, exact = N)` arm cuts its own `N` bytes. A field's `break_on` decides what
//! happens to each value it decodes ([`LoopCtx::field_break`]). [`LoopCtx::unknown_from_input`]
//! and [`LoopCtx::kept_sliced_unknown_arm`] emit the `_` arm, or its `match`, that hands a key no
//! field declares to the `unknown(..)` decoder, reading from the main stream or from a `Sliced`
//! frame's bounded value
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    ErrorKind, LoopCtx,
    framing::{DecodeFraming, UnknownDecoder},
    loop_driver::{CursorSite, ParseSite, TakeInto},
};
use crate::{
    ast::{
        attr::{
            FieldKey, MainContainer, UnknownField, ValueXcoder,
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
use quote::{ToTokens, quote, quote_spanned};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The fewest alternatives of a `K1 | K2 | ..` pattern that `clippy::manual_range_patterns` reads
/// as a range, a repeated key counted each time
const RANGE_LINT_KEYS: usize = 3;

/// The `K1 | K2 | ..` pattern of some keys, as the fields declare them
pub(super) struct KeyPattern<'a> {
    /// The keys, in field order
    keys: Vec<&'a syn::Lit>,
}

/// Where an arm reads its value bytes from
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueSrc {
    /// `&mut __subinput`, the bounded slice the frame body already cut using the header's length,
    /// which a `size(exact = N)` field first checks is `N` long; a known field's decode error is
    /// swallowed via `.ok()`
    HeaderSliced,
    /// [`Self::HeaderSliced`] under the container's `strict`: a decode error, or a byte the decoder
    /// leaves unread, is a malformed value
    StrictSliced,
    /// `input` itself, because the value delimits itself; a decode error goes to
    /// [`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail)
    Inline,
    /// `&mut __subinput`, the `N` bytes the arm itself cuts off `input` for a
    /// `size(implicit, exact = N)` value, whose frame has no length; a decode error is swallowed
    /// via `.ok()`
    Exact(usize),
}

/// What a failed value or length decoder read, which decides whether its failure can be
/// truncation ([`LoopCtx::value_err_arm`])
#[derive(Debug, Clone, Copy)]
pub(super) enum DecodedFrom {
    /// `input`, the main stream, whose end can cut the value short
    Input,
    /// `&mut __subinput`, a `Sliced` frame's bounded value, every byte of which its `take` holds
    Subinput,
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
    /// * `input` - The container, whose `strict` picks the source
    /// * `size` - The field's `size(..)`
    ///
    /// # Returns
    ///
    /// [`Self::StrictSliced`] under `strict`, else [`Self::HeaderSliced`], or `None` for a
    /// `size(implicit)` value ([`Self::implicit`])
    pub(super) fn header_sliced(input: &MainContainer, size: Option<&SizeSpec>) -> Option<Self> {
        let src = match input.attrs.strict {
            Some(_) => Self::StrictSliced,
            None => Self::HeaderSliced,
        };
        Self::implicit(size).is_none().then_some(src)
    }
}

/// [`KeyPattern`] implementation
impl KeyPattern<'_> {
    /// Whether the pattern has no alternative, which no `match` arm or `matches!` can hold
    pub(super) fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Emits `item`, the arm or statement holding the pattern, behind the
    /// `clippy::manual_range_patterns` allow the pattern needs, if any ([`Self::reads_as_range`])
    ///
    /// # Arguments
    ///
    /// * `item` - The arm or statement
    ///
    /// # Returns
    ///
    /// `item`, after the allow when the lint reads the pattern as a range
    pub(super) fn allowed(&self, item: TokenStream) -> TokenStream {
        let allow = self.reads_as_range().then(|| {
            quote! {
                #[allow(
                    clippy::manual_range_patterns,
                    reason = "the pattern is the keys as the fields declare them, which this lint reads as a range",
                )]
            }
        });
        quote! {
            #allow
            #item
        }
    }

    /// Whether `clippy::manual_range_patterns` reads the pattern as a range: [`RANGE_LINT_KEYS`]
    /// or more integer keys, each within `i128`, whose distinct values are consecutive in any
    /// order
    fn reads_as_range(&self) -> bool {
        // --------------------------------------------------
        // the key values, when every key is an integer within `i128`, the lint's own key type
        // --------------------------------------------------
        let values: Option<Vec<i128>> = self
            .keys
            .iter()
            .map(|key| match key {
                syn::Lit::Int(int) => int.base10_parse().ok(),
                _ => None,
            })
            .collect();
        // --------------------------------------------------
        // enough alternatives, whose distinct values are consecutive
        // --------------------------------------------------
        self.keys.len() >= RANGE_LINT_KEYS
            && values.is_some_and(|mut values| {
                values.sort_unstable();
                values.dedup();
                values
                    .windows(2)
                    .all(|pair| matches!(pair, [low, high] if low.checked_add(1) == Some(*high)))
            })
    }
}

/// [`KeyPattern`] implementation of [`FromIterator`], the keys in iteration order
impl<'a> FromIterator<&'a syn::Lit> for KeyPattern<'a> {
    fn from_iter<I: IntoIterator<Item = &'a syn::Lit>>(keys: I) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }
}

/// [`KeyPattern`] implementation of [`ToTokens`]
impl ToTokens for KeyPattern<'_> {
    /// Emits `K1 | K2 | ..`
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let keys = &self.keys;
        tokens.extend(quote! { #(#keys)|* });
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
    /// `.ok() .. .or(__acc.#name)`, or under `strict` rejects ([`ValueSrc::StrictSliced`]); a
    /// failed [`ValueSrc::Inline`] decode leaves the iteration via
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
            // decoder - the explicit codec, or the value type's `DecodeValue` impl; every key has
            // one, as `DecodeCtx::new` returns before arm generation otherwise
            // --------------------------------------------------
            let dec = match codec.dec.as_ref()? {
                ValueXcoder::Explicit(dec) => quote! { #dec },
                ValueXcoder::TraitFallback => {
                    let stream = self.stream;
                    quote! { <#ty as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
                }
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

/// [`LoopCtx`] implementation of a decoder's error arm
impl LoopCtx<'_> {
    /// Emits the `Err` arm of a `match` on a value or length decoder's result
    ///
    /// ```text
    /// Err(__err) => {
    ///     let __err: ContextError = __err;
    ///     #reject
    /// }
    /// ```
    ///
    /// The value is rejected with a [`Self::label`] naming `field`, else the struct's item, and
    /// `decoder` in its reason. One read from the main stream ([`DecodedFrom::Input`]) is
    /// truncation at the end of the input, else malformed
    /// ([`LoopDriver::on_parse_fail`](super::loop_driver::LoopDriver::on_parse_fail)), `(its
    /// <decoder> failed before the end of the input)`. One read from a `Sliced` frame's bounded
    /// value ([`DecodedFrom::Subinput`]) is malformed
    /// ([`LoopDriver::reject_value`](super::loop_driver::LoopDriver::reject_value)), `(its
    /// <decoder> rejected the item's value)`, and never truncation: the `take` already holds
    /// every byte of the frame. The error binding is located at the decoder, so a decoder of
    /// another error type is reported there
    ///
    /// # Arguments
    ///
    /// * `from` - What the decoder read
    /// * `at_dec` - The decoder's location
    /// * `field` - The field the value is for, `None` for the struct's item
    /// * `decoder` - How the reason names the decoder
    ///
    /// # Returns
    ///
    /// The single match arm, every path leaving the iteration
    pub(super) fn value_err_arm(
        self,
        from: DecodedFrom,
        at_dec: proc_macro2::Span,
        field: Option<&syn::Ident>,
        decoder: &str,
    ) -> TokenStream {
        let reject = match from {
            DecodedFrom::Input => {
                let reason = format!("its {decoder} failed before the end of the input");
                let label = self.label(field, &reason);
                self.driver
                    .on_parse_fail(ParseSite::Value { label: &label })
            }
            DecodedFrom::Subinput => {
                let reason = format!("its {decoder} rejected the item's value");
                self.driver
                    .reject_value(&self.label(field, &reason), ErrorKind::MalformedValue)
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
}

/// [`LoopCtx`] implementation of the unknown-key arms
impl LoopCtx<'_> {
    /// Emits the `match` of the `_` arm of a `match` on the frame's key, where the `unknown(..)`
    /// decoder reads the rest of a frame whose key no field declares from the main stream: every
    /// byte after the header, a key + len item's length included
    ///
    /// ```text
    /// match #decoder(#key)(input) {
    ///     Ok(__val) => #push,     // `Ok(_) => ()` when no field keeps the frame
    ///     Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    /// }
    /// ```
    ///
    /// The decoder is called with the key the way a `lengthed` decoder is called with the length
    /// ([`Self::unknown_call`]), on the cursor
    /// [`LoopDriver::read_input`](super::loop_driver::LoopDriver::read_input) hands it. Without a
    /// field to keep it the value is still decoded, which consumes it, and dropped. A failed
    /// decode is truncation or malformed as a [`ValueSrc::Inline`] arm's is
    /// ([`Self::unknown_match`]). The caller's arm checks the frame consumed input: a separate
    /// frame's after this `match` ([`Self::after_key`]), a fused one's after its framing `match`
    /// ([`Self::body`])
    ///
    /// # Arguments
    ///
    /// * `unknown` - The `unknown(dec = ..)` decoder and the field that keeps what it decodes
    ///
    /// # Returns
    ///
    /// The `match`
    pub(super) fn unknown_from_input(self, unknown: UnknownDecoder) -> TokenStream {
        let UnknownDecoder { decoder, keep } = unknown;
        let decoded = self
            .driver
            .read_input(|cursor| Self::unknown_call(decoder, cursor));
        self.unknown_match(decoder, &decoded, keep, DecodedFrom::Input)
    }

    /// Emits the `_` arm of a `Sliced` frame's `match` on its key, where the `unknown(..)` decoder
    /// reads the bounded value of a frame whose key no field declares, for the field that keeps it
    ///
    /// ```text
    /// _ => match #decoder(#key)(&mut __subinput) {
    ///     Ok(__val) => #push,
    ///     Err(__err) => { let __err: ContextError = __err; #reject_value }
    /// },
    /// ```
    ///
    /// The decoder is called with the key ([`Self::unknown_call`]) on the bounded `__subinput`,
    /// where [`Self::unknown_from_input`] hands it the main stream. A failed decode is a malformed
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
        let decoded = Self::unknown_call(decoder, &quote! { &mut __subinput });
        let decode = self.unknown_match(decoder, &decoded, Some(field), DecodedFrom::Subinput);
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
    /// rejected as a value read from `from` ([`Self::value_err_arm`]), labelled with the field,
    /// else with the struct's item
    ///
    /// # Arguments
    ///
    /// * `decoder` - The `unknown(dec = ..)` decoder
    /// * `decoded` - Its call ([`Self::unknown_call`])
    /// * `keep` - The `#[klv(unknown)]` field, or `None` when no field keeps the frame
    /// * `from` - What the decoder read: the main stream, or a `Sliced` frame's bounded value
    ///
    /// # Returns
    ///
    /// The `match`
    fn unknown_match(
        self,
        decoder: &types::XcoderType,
        decoded: &TokenStream,
        keep: Option<&UnknownField>,
        from: DecodedFrom,
    ) -> TokenStream {
        let on_ok = if let Some(field) = keep {
            let push = Self::unknown_push(field);
            quote! { ::core::result::Result::Ok(__val) => #push, }
        } else {
            quote! { ::core::result::Result::Ok(_) => (), }
        };
        let on_err = self.value_err_arm(
            from,
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

    /// Emits the `unknown(..)` decoder called with the frame's key, reading `stream`
    ///
    /// The call, the key included, is located at the decoder, so a decoder of another signature
    /// (one that takes no key, or a key of another type) is reported there
    ///
    /// # Arguments
    ///
    /// * `decoder` - The `unknown(dec = ..)` decoder
    /// * `stream` - The cursor it reads: the main stream's, or `&mut __subinput`
    ///
    /// # Returns
    ///
    /// `#decoder(#key)(#stream)`
    fn unknown_call(decoder: &types::XcoderType, stream: &TokenStream) -> TokenStream {
        let at_dec = helpers::call_site_at(decoder);
        let mut key = DecodeFraming::key();
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
    fn unknown_push(field: &UnknownField) -> TokenStream {
        let UnknownField { name, .. } = field;
        let at_field = helpers::call_site_at(name);
        let mut key = DecodeFraming::key();
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
            "its item length does not match `{}({} = {lit})`",
            symbol::SIZE,
            symbol::EXACT,
        );
        self.driver.reject_exact(&self.label(Some(name), &reason))
    }

    /// Emits the check of a field's `break_on` on the value it just decoded, bound as `__val`
    ///
    /// ```text
    /// match __export::field_break_on(&#break_on, &__val) {
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
    /// accumulator, `Abort` rejects with its message ([`LoopDriver::reject_aborted_value`]), and
    /// `Proceed`, like any later variant, keeps it. The call is located at the fn, which
    /// `__export::field_break_on` bounds by the signature it must have, so a fn of another
    /// signature is reported there, against that bound
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
        let decision = quote_spanned! {helpers::call_site_at(break_on)=>
            ::tinyklv::__export::field_break_on(&#break_on, &__val)
        };
        quote! {
            match #decision {
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
    /// ([`CursorSite::exact_keys`]), in field order, rejecting a key of that width with a length
    /// other than `N`. A width that fields share tells the rejected field apart only once its
    /// check fails. Each check carries the `clippy::manual_range_patterns` allow its pattern
    /// needs, if any ([`KeyPattern::allowed`])
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
        let key = DecodeFraming::key();
        let len = DecodeFraming::len();
        // --------------------------------------------------
        // the exact keys, (key, name, `N` as written), grouped by width in field order
        // --------------------------------------------------
        let mut groups: Vec<(usize, Vec<_>)> = Vec::new();
        for (FieldKey { field, key, .. }, width, lit) in CursorSite::exact_keys(self.input) {
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
            let pattern: KeyPattern = fields.iter().map(|(key, ..)| *key).collect();
            let reject = if let [(_, name, lit)] = fields.as_slice() {
                self.exact_reject(name, lit)
            } else {
                let rejects = fields.iter().map(|(pat, name, lit)| {
                    let reject = self.exact_reject(name, lit);
                    quote! { if ::core::matches!(#key, #pat) { #reject } }
                });
                quote! { #(#rejects)* }
            };
            pattern.allowed(quote! {
                if ::core::matches!(#key, #pattern) && #len != #width {
                    #reject
                }
            })
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
    /// #key => __acc.#name = #dec #len_arg (&mut __subinput).ok() #latebind .or(__acc.#name),
    /// // HeaderSliced, `size(exact = N)`
    /// #key => {
    ///     #exact_check
    ///     __acc.#name = #dec #len_arg (&mut __subinput).ok() #latebind .or(__acc.#name);
    /// },
    /// // Exact(N)
    /// #key => {
    ///     #take(N)
    ///     __acc.#name = #dec #len_arg (&mut __subinput).ok() #latebind .or(__acc.#name);
    /// },
    /// // Inline
    /// #key => {
    ///     let __val = #dec (input);
    ///     match __val {
    ///         Ok(__val) => { __acc.#name = Some(__val) #latebind; }
    ///         Err(__err) => { let __err: ContextError = __err; #on_parse_fail }
    ///     }
    /// },
    /// // Inline, `lengthed`
    /// #key => { #reject },
    /// // HeaderSliced or Exact(N), `break_on`
    /// #key => {
    ///     #prelude
    ///     if let Ok(__val) = #dec #len_arg (&mut __subinput) #latebind { #field_break }
    /// },
    /// // StrictSliced
    /// #key => {
    ///     #exact_check
    ///     match #dec #len_arg (&mut __subinput) {
    ///         Ok(__val) => {
    ///             if Stream::eof_offset(&__subinput) != 0 { #reject_label }
    ///             __acc.#name = Some(__val) #latebind;
    ///         }
    ///         Err(__err) => { let __err: ContextError = __err; #reject_value }
    ///     }
    /// },
    /// // Inline, `break_on`: as Inline, matching `__val #latebind` and the `Ok` arm `#field_break`;
    /// // StrictSliced, `break_on`: likewise
    /// ```
    ///
    /// With `debug` set, the decode result is logged before it is used (a sliced arm then binds
    /// it as `__val` too). The `Inline` error binding and `Some(__val)` are located at the
    /// decoder, so a decoder with another error type (the rule is stated on
    /// `tinyklv::Framing::Inline`) or value type is reported there. An `Inline` frame carries no
    /// length to pass a `lengthed` decoder, so its value is malformed, rejected with a label naming
    /// the field and the cursor still at the value start
    ///
    /// # Returns
    ///
    /// The single match arm
    fn emit(self) -> TokenStream {
        match self.src {
            // --------------------------------------------------
            // sliced: decode the bounded slice, once its length is checked
            // --------------------------------------------------
            ValueSrc::HeaderSliced | ValueSrc::StrictSliced => {
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
                    "an `Inline` item has no length for its `{}({})` decoder",
                    symbol::SIZE,
                    symbol::LENGTHED,
                );
                let label = ctx.label(Some(name), &reason);
                let reject = ctx.driver.reject_label(&label, ErrorKind::WrongLength);
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
        let len = DecodeFraming::len();
        let reject = ctx.exact_reject(self.name, lit);
        Some(quote! {
            if #len != #width {
                #reject
            }
        })
    }

    /// Emits a sliced arm, which decodes `__subinput` after `prelude`: the frame's length check
    /// ([`Self::exact_check`]), or the `take` that cuts an `implicit` value's `N` bytes
    ///
    /// `#len_arg` is the length a `lengthed` decoder is passed ([`SizeSpec::len_source`]): the
    /// frame's length, or `N` under `size(exact = N)`, which the arm's check made the frame's
    /// length or its `take` cut. The value expression and `#len_arg` are located at the decoder,
    /// so a decoder of another value type, or one that takes no length under `size(lengthed)`, is
    /// reported there
    ///
    /// A [`ValueSrc::StrictSliced`] value its decoder fails on is rejected as
    /// [`LoopCtx::value_err_arm`] rejects a sliced one, and one whose decoder leaves a byte of
    /// `__subinput` unread is rejected with a label naming the field
    /// ([`LoopDriver::reject_label`](super::loop_driver::LoopDriver::reject_label)); either leaves
    /// the cursor past the frame
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
            ref dec,
            size,
            ref latebind,
            break_on,
            src,
        } = self;
        let at_dec = helpers::call_site_at(dec);
        // --------------------------------------------------
        // the length a `lengthed` decoder is passed, located at the decoder
        // --------------------------------------------------
        let len_arg = size.and_then(SizeSpec::len_source).map(|source| {
            let mut len = match source {
                LenSource::Frame => TokenTree::from(DecodeFraming::len()),
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
                quote_spanned! {at_dec=> #dec #len_arg (&mut __subinput) },
            ),
            Some(log) => {
                let bind = quote! {
                    let __val = #dec #len_arg (&mut __subinput);
                    #log
                };
                (Some(bind), quote_spanned! {at_dec=> __val })
            }
        };
        // --------------------------------------------------
        // under `strict`, a decode error, or a byte the decoder leaves unread, rejects the value
        // --------------------------------------------------
        if let ValueSrc::StrictSliced = src {
            let (mapped, on_ok) = self.on_ok(at_dec);
            let on_err = ctx.value_err_arm(DecodedFrom::Subinput, at_dec, Some(name), "decoder");
            let unread = ctx.driver.reject_label(
                &ctx.label(
                    Some(name),
                    "its decoder left bytes of the item's value unread",
                ),
                ErrorKind::MalformedValue,
            );
            return quote! {
                #key => {
                    #prelude
                    #bind
                    match #decoded #mapped {
                        ::core::result::Result::Ok(__val) => {
                            if ::tinyklv::__export::winnow::stream::Stream::eof_offset(&__subinput) != 0 {
                                #unread
                            }
                            #on_ok
                        }
                        #on_err
                    }
                },
            };
        }
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
        let at_dec = helpers::call_site_at(&self.dec);
        let (mapped, on_ok) = self.on_ok(at_dec);
        let Self {
            ctx,
            name,
            key,
            dec,
            ..
        } = self;
        let on_err = ctx.value_err_arm(DecodedFrom::Input, at_dec, Some(name), "decoder");
        quote! {
            #key => {
                let __val = #dec (input);
                #log
                match __val #mapped {
                    ::core::result::Result::Ok(__val) => {
                        #on_ok
                    }
                    #on_err
                }
            },
        }
    }

    /// Emits what an arm runs on its decoded value, bound as `__val`: the value is written, or goes
    /// where the field's `break_on` decides once the latebind has mapped the decode result
    ///
    /// `Some(__val)` is located at the decoder, so a decoder of another value type is reported
    /// there
    ///
    /// # Arguments
    ///
    /// * `at_dec` - The decoder's location
    ///
    /// # Returns
    ///
    /// The latebind the decode result is mapped with before its `match`, `None` when the write
    /// maps the value itself, and the statements of the `Ok` arm
    fn on_ok(&self, at_dec: proc_macro2::Span) -> (Option<TokenStream>, TokenStream) {
        let Self {
            ctx,
            name,
            latebind,
            break_on,
            ..
        } = self;
        let some_val = quote_spanned! {at_dec=> ::core::option::Option::Some(__val) };
        match break_on {
            None => (None, quote! { __acc.#name = #some_val #latebind; }),
            Some(break_on) => {
                let write = quote! { __acc.#name = #some_val; };
                (
                    Some(latebind.clone()),
                    ctx.field_break(name, break_on, &write),
                )
            }
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
            quote! { #logger ("\t{}: {:?}", ::core::stringify!(#name), __val); }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::KeyPattern;

    #[test]
    /// Verifies the `clippy::manual_range_patterns` allow is emitted exactly for the keys the lint
    /// reads as a range: three or more integer alternatives, repeats counted, whose distinct values
    /// are consecutive, negative ones included
    fn key_pattern_allows_only_a_lint_range() {
        // --------------------------------------------------
        // each key list as written, and whether it gets the allow
        // --------------------------------------------------
        let cases: [(&[&str], bool); 7] = [
            (&["1", "2", "3"], true),
            (&["3", "1", "2"], true),
            (&["1", "1", "2"], true),
            (&["-1", "0", "1"], true),
            (&["1", "2"], false),
            (&["1", "3", "4"], false),
            (&[r#"b"KLV1""#, r#"b"KLV2""#, r#"b"KLV3""#], false),
        ];
        for (written, allowed) in cases {
            let keys: Vec<syn::Lit> = written
                .iter()
                .map(|key| syn::parse_str(key).expect("a literal key"))
                .collect();
            let pattern: KeyPattern = keys.iter().collect();
            assert_eq!(pattern.reads_as_range(), allowed, "keys {written:?}");
        }
    }
}
