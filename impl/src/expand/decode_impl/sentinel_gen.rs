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
use super::{DecodeCtx, framing::DecodeFraming};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Byte-string sentinels of `1..=SHORT_SENTINEL_MAX` bytes use the inline `memchr`-first-byte
/// seek; longer sentinels (and non-byte-string literals) keep the cached `memmem::Finder`
const SHORT_SENTINEL_MAX: usize = 4;

/// [`DecodeCtx`] implementation of the `SeekSentinel` impl
impl DecodeCtx<'_> {
    /// Generates the `SeekSentinel` trait implementation and its supporting statics
    ///
    /// For a container that declares `#[klv(sentinel = <bytes>)]`, this method emits:
    ///
    /// * A `const __TINYKLV_SENTINEL_LEN_<NAME>: usize` holding the sentinel byte count
    /// * For sentinels longer than `SHORT_SENTINEL_MAX` bytes (and any non-byte-string
    ///   literal): a `static __TINYKLV_SEEKER_<NAME>: LazyLock<memmem::Finder>` for
    ///   zero-cost repeated searches. Short byte-string sentinels skip this static
    ///   entirely and use an inline `memchr`-first-byte + compare scan instead (no
    ///   `LazyLock` deref / searcher dispatch - far cheaper on the small one-shot
    ///   buffers a framed decode sees, while staying SIMD-fast on large streams)
    /// * An `impl SeekSentinel<Stream> for Name` block that locates the sentinel in the
    ///   input (via whichever strategy above), advances past it, decodes the following
    ///   length field with the container's `len` decoder, and returns the value slice
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the sentinel constant, static, and
    /// `SeekSentinel` impl block, or `None` when the container declares no sentinel
    pub(super) fn sentinel_impl(&self) -> Option<proc_macro2::TokenStream> {
        // --------------------------------------------------
        // the sentinel, and the separate length decoder that reads the frame length after it
        // --------------------------------------------------
        let DecodeFraming::Separate {
            len_decoder,
            sentinel: Some(sentinel),
            ..
        } = self.framing
        else {
            return None;
        };
        let name = &self.input.ident;
        let stream = &self.stream;
        let stream_lifetimed = &self.stream_lifetimed;
        let lifetime = &self.lifetime;
        // --------------------------------------------------
        // generics
        // --------------------------------------------------
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        // --------------------------------------------------
        // seeker statics
        // --------------------------------------------------
        let sentinel_len_static_name =
            quote::format_ident!("__TINYKLV_SENTINEL_LEN_{}", name.to_string().to_uppercase());
        let sentinel_seeker_static_name =
            quote::format_ident!("__TINYKLV_SEEKER_{}", name.to_string().to_uppercase());
        // --------------------------------------------------
        // short sentinels (1..=SHORT_SENTINEL_MAX bytes) use inline memchr scan;
        // longer sentinels use the cached memmem::Finder
        // --------------------------------------------------
        let use_inline = matches!(
            sentinel,
            syn::Lit::ByteStr(b) if (1..=SHORT_SENTINEL_MAX).contains(&b.value().len())
        );
        // --------------------------------------------------
        // the seek body: locate the sentinel, advancing `input` past it, or error
        // both branches share the `position + len` advance and the same error
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
                    ::core::option::Option::None => return ::core::result::Result::Err(::tinyklv::__export::labeled_error(
                        input,
                        &checkpoint,
                        concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet"),
                    )),
                };
            }
        } else {
            quote! {
                match #sentinel_seeker_static_name.find(&input) {
                    ::core::option::Option::Some(position) => *input = &input[position + #sentinel_len_static_name..],
                    ::core::option::Option::None => return ::core::result::Result::Err(::tinyklv::__export::labeled_error(
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
        Some(quote! {
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
        })
    }
}
