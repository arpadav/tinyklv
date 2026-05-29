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

/// Byte-string sentinels of `1..=SHORT_SENTINEL_MAX` bytes use the inline `memchr`-first-byte
/// seek; longer sentinels (and non-byte-string literals) keep the cached `memmem::Finder`.
const SHORT_SENTINEL_MAX: usize = 4;

/// Generates the `SeekSentinel` trait implementation and its supporting statics
///
/// For a container that declares `#[klv(sentinel = <bytes>)]`, this function emits:
///
/// * A `const __TINYKLV_SENTINEL_LEN_<NAME>: usize` holding the sentinel byte count
/// * For sentinels longer than `SHORT_SENTINEL_MAX` bytes (and any non-byte-string
///   literal): a `static __TINYKLV_SEEKER_<NAME>: LazyLock<memmem::Finder>` for
///   zero-cost repeated searches. Short byte-string sentinels skip this static
///   entirely and use an inline `memchr`-first-byte + compare scan instead (no
///   `LazyLock` deref / searcher dispatch — far cheaper on the small one-shot
///   buffers a framed decode sees, while staying SIMD-fast on large streams).
/// * An `impl SeekSentinel<Stream> for Name` block that locates the sentinel in the
///   input (via whichever strategy above), advances past it, decodes the following
///   length field, and returns the value slice
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
    // seeker statics
    // --------------------------------------------------
    let sentinel_len_static_name =
        quote::format_ident!("__TINYKLV_SENTINEL_LEN_{}", name.to_string().to_uppercase());
    let sentinel_seeker_static_name =
        quote::format_ident!("__TINYKLV_SEEKER_{}", name.to_string().to_uppercase());
    // --------------------------------------------------
    // short-needle threshold: at/below this byte length we emit an inline
    // `memchr`-first-byte + compare scan instead of the cached
    // `memmem::Finder`. The inline scan has no `LazyLock` deref and no
    // per-call searcher dispatch, so it is dramatically cheaper on the small
    // one-shot buffers a framed decode sees, while staying SIMD-fast (memchr
    // on the first byte) on large streaming buffers. For longer sentinels
    // (e.g. 16-byte MISB universal labels) the `Finder`'s rare-byte heuristic
    // is worth its setup, so those keep the cached finder.
    // --------------------------------------------------
    // a byte-string sentinel of 1..=SHORT_SENTINEL_MAX bytes takes the inline path; everything
    // else (longer byte strings, or a non-byte-string literal) keeps the cached `Finder`. The
    // `1..=` lower bound is load-bearing: it keeps an empty sentinel off the inline path, whose
    // first step indexes `__SENTINEL[0]`.
    let use_inline = matches!(
        sentinel,
        syn::Lit::ByteStr(b) if (1..=SHORT_SENTINEL_MAX).contains(&b.value().len())
    );
    // --------------------------------------------------
    // the seek body: locate the sentinel, advancing `input` past it, or error.
    // both branches share the `position + len` advance and the same error.
    // --------------------------------------------------
    let seek_body = if use_inline {
        quote! {
            const __SENTINEL: &[u8] = #sentinel;
            // `*input` is a `&[u8]` copy; advancing `input` later does not invalidate it
            let __hay: &[u8] = *input;
            let __found = {
                let mut __base = 0usize;
                loop {
                    match ::tinyklv::__export::memchr::memchr(__SENTINEL[0], &__hay[__base..]) {
                        ::core::option::Option::Some(__off) => {
                            let __at = __base + __off;
                            if __hay[__at..].starts_with(__SENTINEL) {
                                break ::core::option::Option::Some(__at);
                            }
                            __base = __at + 1;
                        }
                        ::core::option::Option::None => break ::core::option::Option::None,
                    }
                }
            };
            match __found {
                ::core::option::Option::Some(position) => *input = &input[position + #sentinel_len_static_name..],
                ::core::option::Option::None => return Err(::tinyklv::__export::labeled_error(
                    input,
                    &checkpoint,
                    concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet"),
                )),
            };
        }
    } else {
        quote! {
            match #sentinel_seeker_static_name.find(&input) {
                Some(position) => *input = &input[position + #sentinel_len_static_name..],
                None => return Err(::tinyklv::__export::labeled_error(
                    input,
                    &checkpoint,
                    concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet"),
                )),
            };
        }
    };
    // --------------------------------------------------
    // the cached finder static is only needed for the long-sentinel path
    // --------------------------------------------------
    let seeker_static = if use_inline {
        quote! {}
    } else {
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            #[doc = concat!(" Static seeker for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
            pub(crate) static #sentinel_seeker_static_name: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
                ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));
        }
    };
    // --------------------------------------------------
    // return seek sentinel impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" Static sentinel length for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
        pub(crate) const #sentinel_len_static_name: usize = #sentinel.len();

        #seeker_static

        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", stringify!(#stream), "`]")]
        impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
            fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                let checkpoint = input.checkpoint();
                #seek_body
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
