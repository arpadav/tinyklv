//! Orchestration of the full decode implementation code-generation pass
//!
//! This module is the top-level entry point for generating all decode-related
//! trait implementations for a `#[derive(Klv)]` struct. It derives a [`DecodeCtx`] from the
//! container once, then assembles the impls its methods emit, each from the sub-module that owns
//! it, into a single token stream:
//!
//! * `sentinel_gen` - `SeekSentinel` impl (only when a sentinel is declared, which only a
//!   separate `key`+`len` container can do)
//! * `partial_gen` - partial-packet struct definition, `Partial` impl, and
//!   `TryFrom<Partial>` impl
//! * `decode_partial_gen` - `DecodePartial`, `ResumePartial`, and the one-shot direct `DecodeValue`
//!   impls (`DecodeValue`/`DecodeFrame` decode directly; the streaming `decoder()` path uses the
//!   resumable `Partial` machinery). Both loops are assembled from `framing` (the frame header
//!   and body per framing mode), `loop_driver` (the exits per entry point) and `key_match_gen`
//!   (the per-field arms, and the checks that reject a frame length other than a field's
//!   `size(exact = N)`)
//! * this module - the `decoder()` constructor ([`DecodeCtx::decoder_fn`])
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod decode_partial_gen;
mod framing;
mod key_match_gen;
mod loop_driver;
mod partial_gen;
mod sentinel_gen;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::MainContainer;
use framing::DecodeFraming;
use loop_driver::{CursorSite, LoopDriver};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The fewest alternatives of a `K1 | K2 | ..` pattern that `clippy::manual_range_patterns` reads
/// as a range, a repeated key counted each time
const RANGE_LINT_KEYS: usize = 3;

/// A container that gets decode impls, with everything those impls derive from it
///
/// Built once per derive by [`DecodeCtx::new`]. Each decode impl is a method, defined in the
/// sub-module that generates it
struct DecodeCtx<'a> {
    /// The parsed container, providing the name, generics, fields and attributes
    input: &'a MainContainer<'a>,

    /// The frame header codec(s) and the `unknown(..)` decoder
    framing: DecodeFraming<'a>,

    /// Which cursor the one-shot loop advances, from the fields and the stream
    cursor: CursorSite,

    /// The stream type the impls are parameterised over: the container's `stream`, else `&[u8]`
    stream: syn::Type,

    /// The lifetime `'z` that the `SeekSentinel` and `decoder()` signatures declare, fixed so the
    /// generated impls agree on it
    lifetime: syn::Lifetime,

    /// [`Self::stream`] with [`Self::lifetime`] inserted, as the `SeekSentinel` and `decoder()`
    /// signatures name it
    stream_lifetimed: syn::Type,

    /// The partial-packet struct ident, the container name with `PartialPacket` appended
    /// (e.g. `MyPacket` -> `MyPacketPartialPacket`)
    ///
    /// This naming is load-bearing: the `DecodePartial::Partial` associated type and the
    /// `decoder()` constructor emitted by the decode codegen refer to this exact ident
    partial_name: syn::Ident,

    /// The partial accumulator type, `#partial_name #ty_generics`
    partial_ty: TokenStream,
}

/// A [`LoopDriver`] together with the container, framing and types one emitted loop names
///
/// Its exits are emitted in `loop_driver`, its frame header and body in `framing`, its known-key
/// arms in `key_match_gen`
#[derive(Clone, Copy)]
struct LoopCtx<'a> {
    /// The struct being decoded; each of its keyed fields gets one arm
    input: &'a MainContainer<'a>,

    /// The loop exit policy
    driver: LoopDriver,

    /// The frame header codec(s), which the header reads and whose length binding `take` cuts,
    /// and the `unknown(..)` decoder
    framing: DecodeFraming<'a>,

    /// The stream type the impl is parameterised over, which `take` is instantiated over
    stream: &'a syn::Type,

    /// The partial accumulator type, `#partial_name #ty_generics`, whose `Partial::finalize` the
    /// finalizing exits call
    partial_ty: &'a TokenStream,
}

/// [`DecodeCtx`] implementation
impl<'a> DecodeCtx<'a> {
    /// Derives the decode context of a container
    ///
    /// # Arguments
    ///
    /// * `input` - The fully parsed container, providing field data and all attributes
    ///
    /// # Returns
    ///
    /// The context, or `None` when the struct has no KLV field, or a field, framing or
    /// `unknown(..)` decoder is missing (`allow_unimplemented_decode`), in which case no decode
    /// impl is generated
    fn new(input: &'a MainContainer<'a>) -> Option<Self> {
        // --------------------------------------------------
        // a decoder on every field, and at least one field; else no decode impl
        // --------------------------------------------------
        if !input.every_keyed_field(|f| f.dec.is_some()) {
            return None;
        }
        // --------------------------------------------------
        // frame header codec(s) and the `unknown(..)` decoder; a missing one means no decode
        // impl
        // --------------------------------------------------
        let framing = DecodeFraming::of(input)?;
        // --------------------------------------------------
        // default stream -> &[u8]
        // --------------------------------------------------
        let stream = input.attrs.stream.clone().unwrap_or_else(u8_slice);
        // --------------------------------------------------
        // which cursor the one-shot loop advances
        // --------------------------------------------------
        let cursor = CursorSite::of(input, &stream);
        let lifetime = syn::Lifetime::new("'z", proc_macro2::Span::call_site());
        let stream_lifetimed = insert_lifetime(&stream, lifetime.clone());
        // --------------------------------------------------
        // partial name and the accumulator type it names
        // --------------------------------------------------
        let partial_name = quote::format_ident!("{}PartialPacket", input.ident);
        let (_, ty_generics, _) = input.generics.split_for_impl();
        let partial_ty = quote! { #partial_name #ty_generics };
        Some(Self {
            input,
            framing,
            cursor,
            stream,
            lifetime,
            stream_lifetimed,
            partial_name,
            partial_ty,
        })
    }

    /// Generates the `decoder()` inherent method for a container
    ///
    /// Emits a `pub fn decoder<'z>() -> tinyklv::Decoder<XxxPartialPacket, &'z Stream>`
    /// associated function on the container. This is the idiomatic entry point for
    /// constructing a streaming `tinyklv::Decoder` without needing to name the
    /// partial-packet type directly
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the `impl` block with the `decoder()` method
    fn decoder_fn(&self) -> TokenStream {
        let Self {
            input,
            ref lifetime,
            ref stream_lifetimed,
            ref partial_ty,
            ..
        } = *self;
        let name = &input.ident;
        // --------------------------------------------------
        // generics
        // --------------------------------------------------
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        // --------------------------------------------------
        // emit decoder() inherent method
        // --------------------------------------------------
        quote! {
            #[automatically_derived]
            impl #impl_generics #name #ty_generics #where_clause {
                #[inline(always)]
                #[doc = concat!(" Construct a streaming [`tinyklv::Decoder`] for [`", stringify!(#name), "`].")]
                pub fn decoder<#lifetime>() -> ::tinyklv::Decoder<#partial_ty , #stream_lifetimed> {
                    ::tinyklv::Decoder::new()
                }
            }
        }
    }
}

/// Generates the complete set of decode trait implementations for a container
///
/// Assembles and returns the token stream for every decode-related impl block
/// that `#[derive(Klv)]` emits. The output always includes the partial-packet
/// struct, its `Partial` and `TryFrom` impls, the streaming `DecodePartial` and
/// `ResumePartial` impls, the `decoder()` constructor, and the one-shot
/// `DecodeValue` impl (which decodes directly into the struct in a single pass,
/// rather than delegating to the partial machinery). If the container declares a
/// `sentinel`, a `SeekSentinel` impl is prepended as well
///
/// The container's framing mode selects the frame header and body of both decode loops:
/// separate `key`+`len` decoders, or the fused `keylen` decoder
///
/// # Arguments
///
/// * `input` - The fully parsed container, providing field data and all attributes
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing all decode-related impl blocks,
/// concatenated in dependency order (sentinel first, then partial struct, then impls), or
/// `None` when the struct has no KLV field, or a field, framing or `unknown(..)` decoder is
/// missing (`allow_unimplemented_decode`)
pub(super) fn gen_decode_impl(input: &MainContainer) -> Option<TokenStream> {
    let ctx = DecodeCtx::new(input)?;
    // --------------------------------------------------
    // seek implementation
    // --------------------------------------------------
    let seek_if_sentinel = ctx.sentinel_impl();
    // --------------------------------------------------
    // partial struct def, Partial impl, TryFrom impl
    // --------------------------------------------------
    let partial_struct_def = ctx.partial_struct();
    let partial_struct_impl = ctx.partial_impl();
    let try_from_partial_struct_impl = ctx.try_from_partial_impl();
    // --------------------------------------------------
    // DecodePartial (framing-agnostic: delegates to ResumePartial), ResumePartial and one-shot
    // DecodeValue
    // --------------------------------------------------
    let decode_partial_impl = ctx.decode_partial_impl();
    let resume_partial_impl = ctx.resume_partial_impl();
    let decode_value_impl = ctx.decode_value_impl();
    // --------------------------------------------------
    // decoder() constructor
    // --------------------------------------------------
    let decoder_fn_impl = ctx.decoder_fn();
    // --------------------------------------------------
    // assemble and return all impls
    // --------------------------------------------------
    Some(quote! {
        #seek_if_sentinel
        #partial_struct_def
        #partial_struct_impl
        #try_from_partial_struct_impl
        #decoder_fn_impl
        #resume_partial_impl
        #decode_partial_impl
        #decode_value_impl
    })
}

/// Inserts a named lifetime into a [`syn::Type`], producing a reference type
///
/// When `ty` is already a reference (`&T` or `&mut T`), the lifetime is placed
/// on the existing reference. When `ty` is any other form (e.g. `[u8]`, a path
/// type), it is wrapped in a new shared reference `&'lifetime ty`. This is used
/// to produce the stream-with-lifetime form required by `SeekSentinel` and
/// `decoder()` function signatures
///
/// # Arguments
///
/// * `ty` - The base type to attach the lifetime to
/// * `lifetime` - The lifetime to attach, e.g. `'z`
///
/// # Returns
///
/// A new [`syn::Type`] with the named lifetime attached
fn insert_lifetime(ty: &syn::Type, lifetime: syn::Lifetime) -> syn::Type {
    match ty {
        syn::Type::Reference(ty_ref) => syn::Type::Reference(syn::TypeReference {
            and_token: Default::default(),
            lifetime: Some(lifetime),
            mutability: ty_ref.mutability,
            elem: ty_ref.elem.clone(),
        }),
        _ => syn::Type::Reference(syn::TypeReference {
            and_token: Default::default(),
            lifetime: Some(lifetime),
            mutability: None,
            elem: Box::new(ty.clone()),
        }),
    }
}

/// Emits the `K1 | K2 | ..` pattern of `keys`, and the `clippy::manual_range_patterns` allow the
/// arm or statement holding it needs, if any
///
/// The pattern is the keys as the fields declare them, which the lint reads as a range when they
/// are [`RANGE_LINT_KEYS`] or more integers, each within `i128`, whose distinct values are
/// consecutive in any order; only then is the allow emitted
///
/// # Arguments
///
/// * `keys` - The keys, in field order
///
/// # Returns
///
/// The pattern, and the allow, `None` when the lint does not read the keys as a range
fn key_pattern(keys: &[&syn::Lit]) -> (TokenStream, Option<TokenStream>) {
    // --------------------------------------------------
    // the key values, when every key is an integer within `i128`, the lint's own key type
    // --------------------------------------------------
    let values: Option<Vec<i128>> = keys
        .iter()
        .map(|key| match key {
            syn::Lit::Int(int) => int.base10_parse().ok(),
            _ => None,
        })
        .collect();
    // --------------------------------------------------
    // the allow, for enough alternatives whose distinct values are consecutive
    // --------------------------------------------------
    let run = keys.len() >= RANGE_LINT_KEYS
        && values.is_some_and(|mut values| {
            values.sort_unstable();
            values.dedup();
            values
                .windows(2)
                .all(|pair| matches!(pair, [low, high] if low.checked_add(1) == Some(*high)))
        });
    let allow = run.then(|| {
        quote! {
            #[allow(
                clippy::manual_range_patterns,
                reason = "the pattern is the keys as the fields declare them, which this lint reads as a range",
            )]
        }
    });
    (quote! { #(#keys)|* }, allow)
}

/// Constructs the default stream type `&[u8]` as a [`syn::Type`]
///
/// Used when the container's `stream = ..` attribute is absent. Builds the type programmatically
/// to avoid a `syn::parse_str` call that would require an `unwrap`
///
/// # Returns
///
/// A [`syn::Type`] equivalent to `&[u8]`
fn u8_slice() -> syn::Type {
    syn::Type::Reference(syn::TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(syn::Type::Slice(syn::TypeSlice {
            bracket_token: Default::default(),
            elem: Box::new(syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::parse_quote! { u8 },
            })),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::key_pattern;

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
            let (_, allow) = key_pattern(&keys.iter().collect::<Vec<_>>());
            assert_eq!(allow.is_some(), allowed, "keys {written:?}");
        }
    }
}
