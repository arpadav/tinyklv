//! Orchestration of the full decode implementation code-generation pass
//!
//! This module is the top-level entry point for generating all decode-related
//! trait implementations for a `#[derive(Klv)]` struct. It drives the
//! sub-modules in order and assembles their outputs into a single token stream:
//!
//! * `sentinel_gen` - `SeekSentinel` impl (only when a sentinel is declared)
//! * `partial_gen` - partial-packet struct definition, `Partial` impl, and
//!   `TryFrom<Partial>` impl
//! * `decode_partial_gen` - `DecodePartial`, `ResumePartial`, and the one-shot direct `DecodeValue`
//!   impls (`DecodeValue`/`DecodeFrame` decode directly; the streaming `decoder()` path uses the
//!   resumable `Partial` machinery)
//! * local helper - the `decoder()` constructor
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod constants;
mod decode_partial_gen;
mod key_match_gen;
mod partial_gen;
mod sentinel_gen;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    ast::{attr::MainContainer, types},
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

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
/// # Arguments
///
/// * `input` - The fully parsed container, providing field data and all attributes
/// * `key_decoder` - Token expression used to parse each TLV key from the stream
/// * `len_decoder` - Token expression used to parse each TLV length from the stream
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing all decode-related impl blocks,
/// concatenated in dependency order (sentinel first, then partial struct, then impls)
pub(crate) fn gen_decode_impl(
    input: &MainContainer,
    key_decoder: &types::XcoderType,
    len_decoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // get name and partial name
    // --------------------------------------------------
    let name = &input.ident;
    let partial_name = constants::create_partial_name(name);

    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let stream = input.attrs.stream.clone().unwrap_or(helpers::u8_slice());
    let lifetime = constants::create_lifetime();
    let stream_lifetimed = helpers::insert_lifetime(&stream, lifetime.clone());

    // --------------------------------------------------
    // seek implementation
    // --------------------------------------------------
    let sentinel = input.attrs.sentinel.as_ref();
    let seek_if_sentinel = match sentinel {
        Some(sentinel) => sentinel_gen::gen_sentinel_impl(
            name,
            sentinel,
            &stream,
            &stream_lifetimed,
            &lifetime,
            len_decoder,
            input.generics,
        ),
        None => quote! {},
    };

    // --------------------------------------------------
    // * the init sequence at beginning of decode, partial struct definition
    // * impl Partial for partial struct, which converts
    //   the partial packet -> final struct
    // * impl TryFrom<partial struct> for final struct
    // --------------------------------------------------
    let partial_struct_def = partial_gen::gen_partial_struct(
        name,
        &partial_name,
        &input._original.vis,
        input.generics,
        &input.data,
    );
    let partial_struct_impl =
        partial_gen::gen_partial_impl(name, &partial_name, input.generics, &input.data);
    let try_from_partial_struct_impl =
        partial_gen::gen_try_from_partial_impl(name, &partial_name, input.generics);
    // --------------------------------------------------
    // partial implementations
    // --------------------------------------------------
    let decode_partial_impl =
        decode_partial_gen::gen_decode_partial_impl(name, &partial_name, &stream, input.generics);
    let resume_partial_impl = decode_partial_gen::gen_resume_partial_impl(
        input,
        name,
        &partial_name,
        &stream,
        key_decoder,
        len_decoder,
    );
    // --------------------------------------------------
    // main decoder functions
    // --------------------------------------------------
    let decoder_fn_impl = gen_decoder_fn(
        name,
        &partial_name,
        &stream_lifetimed,
        lifetime,
        input.generics,
    );
    let decode_value_impl = decode_partial_gen::gen_decode_value(
        input,
        name,
        &partial_name,
        &stream,
        key_decoder,
        len_decoder,
    );
    // --------------------------------------------------
    // return em all
    // --------------------------------------------------
    quote! {
        #seek_if_sentinel
        #partial_struct_def
        #partial_struct_impl
        #try_from_partial_struct_impl
        #decoder_fn_impl
        #resume_partial_impl
        #decode_partial_impl
        #decode_value_impl
    }
}

/// Generates the `decoder()` inherent method for a container
///
/// Emits a `pub fn decoder<'z>() -> tinyklv::Decoder<XxxPartialPacket, &'z Stream>`
/// associated function on the container. This is the idiomatic entry point for
/// constructing a streaming [`tinyklv::Decoder`] without needing to name the
/// partial-packet type directly
///
/// # Arguments
///
/// * `name` - The container ident the method is generated on
/// * `partial_name` - The partial-packet struct ident used as the `Decoder` type parameter
/// * `stream_lifetimed` - The stream type with the generated lifetime already inserted
/// * `lifetime` - The lifetime token (e.g. `'z`) to add as a generic parameter
/// * `generics` - Generic parameters from the original struct definition
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the `impl` block with the `decoder()` method
fn gen_decoder_fn(
    name: &syn::Ident,
    partial_name: &syn::Ident,
    stream_lifetimed: &syn::Type,
    lifetime: proc_macro2::TokenStream,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // return
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            #[inline(always)]
            #[doc = concat!(" Construct a streaming [`tinyklv::Decoder`] for [`", stringify!(#name), "`].")]
            pub fn decoder<#lifetime>() -> ::tinyklv::Decoder<#partial_name #ty_generics , #stream_lifetimed> {
                ::tinyklv::Decoder::new()
            }
        }
    }
}
