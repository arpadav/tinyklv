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
use super::{DecodeCtx, ErrorKind, framing::DecodeFraming};

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
    ///   length field with the container's `len` decoder, and returns the value slice. A
    ///   sentinel whose length fails short of the end of the input begins no packet, so the
    ///   search resumes one byte past its start. On an error `input` is left at the found
    ///   sentinel, or on a miss at the last `len - 1` bytes, the only ones that may still
    ///   begin a sentinel. A miss is [`ErrorKind::MissingSentinel`]; a length or a body that
    ///   the end of the input cuts is [`ErrorKind::Truncated`], as a value failing there is
    ///   (DES-12)
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
        let (missing, truncated) = (ErrorKind::MissingSentinel, ErrorKind::Truncated);
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
        // the search: the sentinel's offset in `__hay`, or `None`
        // --------------------------------------------------
        let find = if use_inline {
            quote! {
                let __found = {
                    const __SENTINEL: &[u8] = #sentinel;
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
            }
        } else {
            quote! {
                let __found = #sentinel_seeker_static_name.find(__hay);
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
                #[doc = ::core::concat!(" Static seeker for [`", ::core::stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
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
            #[doc = ::core::concat!(" Static sentinel length for [`", ::core::stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
            pub(crate) const #sentinel_len_static_name: usize = #sentinel.len();

            #seeker_static

            #[automatically_derived]
            #[doc(hidden)]
            #[doc = ::core::concat!(" [`", ::core::stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", ::core::stringify!(#stream), "`]")]
            impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
                fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                    let checkpoint = input.checkpoint();
                    // `*input` is a `&[u8]` copy; advancing `input` later does not invalidate it
                    let mut __hay: &[u8] = *input;
                    loop {
                        #find
                        let ::core::option::Option::Some(position) = __found else {
                            // only the last `len - 1` bytes may still begin a sentinel
                            *input = &__hay[__hay.len().saturating_sub(#sentinel_len_static_name.saturating_sub(1))..];
                            return ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                                input,
                                &checkpoint,
                                ::core::concat!("Unable to find recognition sentinel for `", ::core::stringify!(#name), "` packet"),
                                #missing,
                            ));
                        };
                        let mut __rest: &[u8] = &__hay[position + #sentinel_len_static_name..];
                        match #len_decoder
                            .context(::tinyklv::__export::winnow::error::StrContext::Label(
                                ::core::concat!("Unable to parse packet length for `", ::core::stringify!(#name), "` packet")
                            ))
                            .parse_next(&mut __rest)
                        {
                            // `as usize` truncates a decoded length only if it exceeds
                            // `usize::MAX`, which cannot occur on 64-bit targets for any
                            // supported length codec
                            ::core::result::Result::Ok(packet_len) => match __rest.split_at_checked(packet_len as usize) {
                                ::core::option::Option::Some((__body, __tail)) => {
                                    *input = __tail;
                                    return ::core::result::Result::Ok(__body);
                                }
                                // the body is cut: the packet is not all here yet
                                ::core::option::Option::None => {
                                    *input = &__hay[position..];
                                    return ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                                        input,
                                        &checkpoint,
                                        ::core::concat!("Unable to read packet body for `", ::core::stringify!(#name), "` packet"),
                                        #truncated,
                                    ));
                                }
                            },
                            // the length is cut: the packet is not all here yet
                            ::core::result::Result::Err(__e) if __rest.is_empty() => {
                                *input = &__hay[position..];
                                return ::core::result::Result::Err(::tinyklv::__export::with_kind(__e, #truncated));
                            }
                            // the length is invalid: this sentinel begins no packet
                            ::core::result::Result::Err(_) => __hay = &__hay[position + 1..],
                        }
                    }
                }
            }
        })
    }
}
