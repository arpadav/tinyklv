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
use crate::expand::helpers;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{quote, quote_spanned};

/// [`DecodeCtx`] implementation of the `SeekSentinel` impl
impl DecodeCtx<'_> {
    /// Generates the `SeekSentinel` trait implementation
    ///
    /// For a container that declares `#[klv(sentinel = <bytes>)]`, this method emits an
    /// `impl SeekSentinel<Stream> for Name` block whose `seek_unlabeled` seeks, declaring,
    /// private to it:
    ///
    /// * A `const __TINYKLV_SENTINEL_LEN: usize` holding the sentinel byte count
    /// * A `static __TINYKLV_SEEKER: LazyLock<memmem::Finder>`, built on the first search that
    ///   does not find the sentinel at the start of the input
    ///
    /// It locates the sentinel in the input, compared at its start before the finder searches
    /// the rest (the start is where back-to-back frames put every sentinel), advances past it,
    /// decodes the following length field, a `usize`, with the container's `len` decoder, and
    /// returns the value slice. On an error `input` is left at the found sentinel, where the
    /// packet starts, or on a miss at the last `len - 1` bytes, the only ones that may still begin
    /// a sentinel. The error is a kind alone, which builds nothing: a miss is
    /// [`ErrorKind::MissingSentinel`]; a length its decoder tags [`ErrorKind::MalformedValue`]
    /// keeps that kind; any other length failure, and a body that the end of the input cuts, is
    /// [`ErrorKind::Truncated`], so a length decoder that stops short of the end of the input on
    /// a cut read is waited on too (DES-46)
    ///
    /// Its `seek_sentinel` runs that seek and labels a failure: a miss, a cut body, or, read
    /// again from the sentinel, the length decoder's own error, relabelled to name the struct's
    /// packet length first, each tagged as the seek tagged it
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
        let (missing, truncated, malformed) = (
            ErrorKind::MissingSentinel,
            ErrorKind::Truncated,
            ErrorKind::MalformedValue,
        );
        let stream_lifetimed = &self.stream_lifetimed;
        let lifetime = &self.lifetime;
        // --------------------------------------------------
        // the packet length's read, a `usize` as every frame length is, located at its decoder
        // so that a decoder of another length type is reported there
        // --------------------------------------------------
        let parse_len = quote_spanned! {helpers::call_site_at(len_decoder)=>
            let __parsed: ::core::result::Result<::core::primitive::usize, _> =
                ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, &mut __rest);
        };
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
            #[doc = ::core::concat!(" [`", ::core::stringify!(#name), "`] implementation of [`tinyklv::traits::SeekSentinel`] for [`", ::core::stringify!(#stream), "`]")]
            impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
                fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                    let __checkpoint = ::tinyklv::__export::winnow::stream::Stream::checkpoint(input);
                    let __kind = match <Self as ::tinyklv::traits::SeekSentinel<#stream>>::seek_unlabeled(input) {
                        ::core::result::Result::Ok(__body) => return ::core::result::Result::Ok(__body),
                        ::core::result::Result::Err(__kind) => __kind,
                    };
                    if __kind == #missing {
                        return ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                            input,
                            &__checkpoint,
                            ::core::concat!("`", ::core::stringify!(#name), "` packet (no recognition sentinel in the input)"),
                            #missing,
                        ));
                    }
                    // the seek stopped at the sentinel; its length, read again, says what failed
                    let mut __rest: &[::core::primitive::u8] = &input[#sentinel.len()..];
                    match ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, &mut __rest) {
                        ::core::result::Result::Ok(_) => ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                            input,
                            &__checkpoint,
                            ::core::concat!("`", ::core::stringify!(#name), "` packet (its body runs past the end of the input)"),
                            #truncated,
                        )),
                        // named as the struct's packet length first, then as the decoder names it
                        ::core::result::Result::Err(__e) => ::core::result::Result::Err(::tinyklv::__export::relabeled_error(
                            input,
                            &__checkpoint,
                            ::core::concat!("`", ::core::stringify!(#name), "` packet length"),
                            __e,
                            __kind,
                        )),
                    }
                }

                // inlined into `seek_sentinel`, whose success path is this seek's: out of line it
                // costs a one-shot `decode_frame` 16 more instructions a packet (measured)
                #[inline(always)]
                fn seek_unlabeled<#lifetime>(input: &mut #stream_lifetimed) -> ::core::result::Result<#stream_lifetimed, ::tinyklv::ErrorKind> {
                    const __TINYKLV_SENTINEL_LEN: ::core::primitive::usize = #sentinel.len();
                    static __TINYKLV_SEEKER: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
                        ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));
                    // `*input` is a `&[u8]` copy; advancing `input` later does not invalidate it
                    let __hay: &[::core::primitive::u8] = *input;
                    // the sentinel's offset in `__hay`; `AsRef` takes a byte string or a string
                    let __found = if __hay.starts_with(::core::convert::AsRef::<[::core::primitive::u8]>::as_ref(#sentinel)) {
                        ::core::option::Option::Some(0)
                    } else {
                        __TINYKLV_SEEKER.find(__hay)
                    };
                    let ::core::option::Option::Some(__position) = __found else {
                        // only the last `len - 1` bytes may still begin a sentinel
                        *input = &__hay[__hay.len().saturating_sub(__TINYKLV_SENTINEL_LEN.saturating_sub(1))..];
                        return ::core::result::Result::Err(#missing);
                    };
                    let mut __rest: &[::core::primitive::u8] = &__hay[__position + __TINYKLV_SENTINEL_LEN..];
                    #parse_len
                    match __parsed {
                        ::core::result::Result::Ok(__len) => match __rest.split_at_checked(__len) {
                            ::core::option::Option::Some((__body, __tail)) => {
                                *input = __tail;
                                ::core::result::Result::Ok(__body)
                            }
                            // the body is cut: the packet is not all here yet
                            ::core::option::Option::None => {
                                *input = &__hay[__position..];
                                ::core::result::Result::Err(#truncated)
                            }
                        },
                        // the length is cut, or malformed when its decoder says so: either way
                        // the packet starts at the sentinel
                        ::core::result::Result::Err(__e) => {
                            *input = &__hay[__position..];
                            match ::tinyklv::ErrorKind::of(&__e) {
                                ::core::option::Option::Some(#malformed) => ::core::result::Result::Err(#malformed),
                                _ => ::core::result::Result::Err(#truncated),
                            }
                        }
                    }
                }
            }
        })
    }
}
