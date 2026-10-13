//! Code generation for the `SeekSentinel` trait implementation
//!
//! Emits the `SeekSentinel` impl, with its per-container `memmem` seeker private to it, for
//! containers that declare a `sentinel = ..` attribute
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

/// [`DecodeCtx`] implementation of the `SeekSentinel` impl
impl DecodeCtx<'_> {
    /// Generates the `SeekSentinel` trait implementation
    ///
    /// For a container that declares `#[klv(sentinel = <bytes>)]`, this method emits an
    /// `impl SeekSentinel<Stream> for Name` block whose `seek_sentinel` declares, private to it:
    ///
    /// * A `const __TINYKLV_SENTINEL_LEN: usize` holding the sentinel byte count
    /// * A `static __TINYKLV_SEEKER: LazyLock<memmem::Finder>`, built on the first search that
    ///   does not find the sentinel at the start of the input
    ///
    /// It locates the sentinel in the input, compared at its start before the finder searches
    /// the rest (the start is where back-to-back frames put every sentinel), advances past it,
    /// decodes the following length field with the container's `len` decoder, and returns the
    /// value slice. A sentinel whose length fails short of the end of the input begins no packet,
    /// so the search resumes one byte past its start. On an error `input` is left at the found
    /// sentinel, or on a miss at the last `len - 1` bytes, the only ones that may still begin a
    /// sentinel. A miss is [`ErrorKind::MissingSentinel`]; a length or a body that the end of the
    /// input cuts is [`ErrorKind::Truncated`], as a value failing there is (DES-12)
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the `SeekSentinel` impl block, or `None` when
    /// the container declares no sentinel
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
        // return seek sentinel impl
        // --------------------------------------------------
        Some(quote! {
            #[automatically_derived]
            #[doc(hidden)]
            #[doc = ::core::concat!(" [`", ::core::stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", ::core::stringify!(#stream), "`]")]
            impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
                fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                    const __TINYKLV_SENTINEL_LEN: usize = #sentinel.len();
                    static __TINYKLV_SEEKER: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
                        ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));
                    let checkpoint = input.checkpoint();
                    // `*input` is a `&[u8]` copy; advancing `input` later does not invalidate it
                    let mut __hay: &[u8] = *input;
                    loop {
                        // the sentinel's offset in `__hay`; `AsRef` takes a byte string or a string
                        let __found = if __hay.starts_with(::core::convert::AsRef::<[u8]>::as_ref(#sentinel)) {
                            ::core::option::Option::Some(0)
                        } else {
                            __TINYKLV_SEEKER.find(__hay)
                        };
                        let ::core::option::Option::Some(position) = __found else {
                            // only the last `len - 1` bytes may still begin a sentinel
                            *input = &__hay[__hay.len().saturating_sub(__TINYKLV_SENTINEL_LEN.saturating_sub(1))..];
                            return ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                                input,
                                &checkpoint,
                                ::core::concat!("Unable to find recognition sentinel for `", ::core::stringify!(#name), "` packet"),
                                #missing,
                            ));
                        };
                        let mut __rest: &[u8] = &__hay[position + __TINYKLV_SENTINEL_LEN..];
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
