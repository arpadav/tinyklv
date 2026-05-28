//! Code generation for the `SeekSentinel` trait implementation
//!
//! Emits the `SeekSentinel` impl, associated statics, and the per-container
//! `memmem` seeker for containers that declare a `sentinel = ..` attribute
//! The sentinel mechanism allows a streaming decoder to scan a raw byte buffer
//! for a known marker byte sequence before attempting to decode a packet
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates the `SeekSentinel` trait implementation and its supporting statics
///
/// For a container that declares `#[klv(sentinel = <bytes>)]`, this function emits:
///
/// * A `const __TINYKLV_SENTINEL_LEN_<NAME>: usize` holding the sentinel byte count
/// * A `static __TINYKLV_SEEKER_<NAME>: LazyLock<memmem::Finder>` for zero-cost
///   repeated searches
/// * An `impl SeekSentinel<Stream> for Name` block that uses the seeker to locate
///   the sentinel in the input, advances past it, decodes the following length field,
///   and returns the value slice
///
/// # Arguments
///
/// * `name` - The container ident the impl is generated for
/// * `sentinel` - The sentinel literal (e.g. `b"HEARTBEAT"`) from the container attribute
/// * `stream` - The stream type parameterising the impl (e.g. `&[u8]`)
/// * `stream_lifetimed` - The stream type with the generated lifetime inserted
/// * `lifetime` - The lifetime token used in the impl signature (e.g. `'z`)
/// * `len_decoder` - Token expression used to decode the packet length that follows
///   the sentinel in the stream
/// * `generics` - Generic parameters from the original struct definition
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the sentinel constant, static, and
/// `SeekSentinel` impl block
pub(super) fn gen_sentinel_impl(
    name: &syn::Ident,
    sentinel: &syn::Lit,
    stream: &syn::Type,
    stream_lifetimed: &syn::Type,
    lifetime: &proc_macro2::TokenStream,
    len_decoder: &types::XcoderType,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // seeker using memchr
    // --------------------------------------------------
    let sentinel_len_static_name =
        quote::format_ident!("__TINYKLV_SENTINEL_LEN_{}", name.to_string().to_uppercase());
    let sentinel_seeker_static_name =
        quote::format_ident!("__TINYKLV_SEEKER_{}", name.to_string().to_uppercase());
    // --------------------------------------------------
    // return seek sentinel impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" Static sentinel length for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
        pub(crate) const #sentinel_len_static_name: usize = #sentinel.len();

        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" Static seeker for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
        pub(crate) static #sentinel_seeker_static_name: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
            ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));

        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", stringify!(#stream), "`]")]
        impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
            fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                let checkpoint = input.checkpoint();
                match #sentinel_seeker_static_name.find(&input) {
                    Some(position) => *input = &input[position + #sentinel_len_static_name..],
                    None => return Err(::tinyklv::__export::labeled_error(
                        input,
                        &checkpoint,
                        concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet"),
                    )),
                };
                // `as usize` truncates a decoded length only if it exceeds
                // `usize::MAX`, which cannot occur on 64-bit targets for any
                // supported length codec
                let packet_len = #len_decoder
                    .context(::tinyklv::__export::winnow::error::StrContext::Label(
                        concat!("Unable to parse packet length for `", stringify!(#name), "` packet")
                    ))
                    .parse_next(input)? as usize;
                ::tinyklv::__export::winnow::token::take(packet_len).parse_next(input)
            }
        }
    }
}
