//! The frame header and frame body of a generated decode loop
//!
//! [`DecodeFraming`] is the codegen side of the container's framing mode. Each loop iteration reads
//! one frame header ([`LoopCtx::header`]) and then decodes the value that follows it
//! ([`LoopCtx::body`]), both from the loop's own [`DecodeFraming`]:
//!
//! * [`DecodeFraming::Separate`] reads `key` then `len`, takes `len` bytes and dispatches every
//!   field against them
//! * [`DecodeFraming::Fused`] reads `(key, Framing)` from the `keylen` codec and branches on the
//!   framing at run time: `Sliced` is the separate body, `Inline` dispatches every field
//!   against the main stream ([`ValueSrc::Inline`]). A frame that moved the cursor nowhere is
//!   rejected after its value
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    LoopCtx,
    key_match_gen::ValueSrc,
    loop_driver::{LoopDriver, ParseSite, TakeInto},
};
use crate::{
    ast::{attr::container::ContainerMode, types},
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

/// The container's frame header codec(s), carrying the borrowed decoder token(s)
///
/// Mirrors [`ContainerMode`] on the codegen side
#[derive(Debug, Clone, Copy)]
pub(super) enum DecodeFraming<'a> {
    /// Separate key and length decoders; every value is sliced via `take(len)`
    Separate {
        /// Token expression decoding each frame's key
        key_decoder: &'a types::XcoderType,
        /// Token expression decoding each frame's length
        len_decoder: &'a types::XcoderType,
        /// The sentinel the `SeekSentinel` impl scans for ahead of the frame length, if any
        sentinel: Option<&'a syn::Lit>,
    },
    /// One decoder returning `(key, Framing)`, so the framing can depend on the key
    Fused {
        /// Token expression decoding each frame header into `(key, Framing)`
        keylen_decoder: &'a types::XcoderType,
    },
}

/// [`DecodeFraming`] implementation
impl<'a> DecodeFraming<'a> {
    /// Borrows the decoder token(s) of a container's framing mode
    ///
    /// # Arguments
    ///
    /// * `mode` - The container's validated framing mode
    ///
    /// # Returns
    ///
    /// The framing, or `None` when a decoder is missing (`allow_unimplemented_decode`), in which
    /// case no decode impl is generated
    pub(super) fn from_mode(mode: &'a ContainerMode) -> Option<Self> {
        match mode {
            ContainerMode::Separate {
                key, len, sentinel, ..
            } => Some(Self::Separate {
                key_decoder: key.dec.as_ref()?,
                len_decoder: len.dec.as_ref()?,
                sentinel: sentinel.as_ref(),
            }),
            ContainerMode::Fused { keylen, .. } => Some(Self::Fused {
                keylen_decoder: keylen.dec.as_ref()?,
            }),
        }
    }

    /// The binding the frame header puts the decoded key in, which every later read of the key
    /// names
    ///
    /// # Returns
    ///
    /// `key` (separate) or `__key` (fused), see [`Self::binding`]
    pub(super) fn key(self) -> syn::Ident {
        self.binding("key")
    }

    /// The binding the frame's value length is put in, which every `take` of the value and every
    /// `size(exact = N)` check reads, and which a `lengthed` decoder without `exact` is passed
    ///
    /// # Returns
    ///
    /// `len` (separate) or `__len` (fused), see [`Self::binding`]
    pub(super) fn len(self) -> syn::Ident {
        self.binding("len")
    }

    /// The binding a known-key arm puts its value decoder's result in
    ///
    /// # Returns
    ///
    /// `val` (separate) or `__val` (fused), see [`Self::binding`]
    pub(super) fn val(self) -> syn::Ident {
        self.binding("val")
    }

    /// Names the key, the length or the decode result the frame binds
    ///
    /// The fused frame binds these three under an internal name, so a user `const` of the same
    /// name in scope is neither turned into a pattern nor reported against the derive. The
    /// separate frame binds the plain name, where such a `const` still becomes a pattern
    ///
    /// # Arguments
    ///
    /// * `name` - The plain name of the local
    ///
    /// # Returns
    ///
    /// `name` (separate) or `__name` (fused)
    fn binding(self, name: &str) -> syn::Ident {
        match self {
            Self::Separate { .. } => syn::Ident::new(name, proc_macro2::Span::call_site()),
            Self::Fused { .. } => quote::format_ident!("__{name}"),
        }
    }

    /// Emits the `debug` log line for the frame header just read
    ///
    /// # Returns
    ///
    /// A log of [`Self::key`] with [`Self::len`] (separate) or `__framing` (fused)
    pub(super) fn debug_line(self) -> TokenStream {
        let logger = crate::expand::logger::debug_logger();
        let key = self.key();
        let len = self.len();
        match self {
            Self::Separate { .. } => quote! { #logger ("key: {}, len: {}", #key, #len); },
            Self::Fused { .. } => quote! { #logger ("key: {}, framing: {:?}", #key, __framing); },
        }
    }

    /// Whether [`LoopCtx::body`]'s unknown-key arm is a `break`, one of the reports
    /// [`DecodeCtx::breaks`](super::DecodeCtx::breaks) ORs
    ///
    /// # Returns
    ///
    /// `true` for a fused frame, whose unknown `Inline` key ends the loop
    pub(super) fn unknown_key_breaks(self) -> bool {
        matches!(self, Self::Fused { .. })
    }
}

/// [`LoopCtx`] implementation of the frame header and body
impl LoopCtx<'_> {
    /// Emits the frame header read, binding [`DecodeFraming::key`] and either
    /// [`DecodeFraming::len`] or `__framing`
    ///
    /// The separate header keeps its own failure rules. Streaming: a key decoder that fails
    /// without consuming a byte is truncation, one that consumed bytes is malformed and finalizes;
    /// any length failure is truncation. One-shot: any failure rewinds and stops. The fused header
    /// follows the rules stated on `tinyklv::Framing`: a failure is truncation
    /// ([`LoopDriver::on_parse_fail`] with [`ParseSite::Header`]). It also binds `__frame_offset`,
    /// the input position at the frame start, which [`LoopCtx::body`] checks progress against
    ///
    /// The fused header result is bound with the type `Result<(_, Framing), _>`, located at the
    /// `keylen` decoder, so a decoder with the wrong return type is reported at the decoder
    ///
    /// # Returns
    ///
    /// The header statements
    pub(super) fn header(self) -> TokenStream {
        let Self {
            driver,
            framing,
            partial_ty,
            ..
        } = self;
        let key = framing.key();
        let len = framing.len();
        let truncated = driver.truncated();
        match (framing, driver) {
            // --------------------------------------------------
            // separate key and length, streaming: a key failing without consuming a byte,
            // or any length failure, is truncation; a key that consumed bytes finalizes
            // --------------------------------------------------
            (
                DecodeFraming::Separate {
                    key_decoder,
                    len_decoder,
                    ..
                },
                LoopDriver::Streaming,
            ) => {
                let clean_eof = driver.clean_eof();
                quote! {
                    let __key_offset_before = input.eof_offset();
                    let ::core::result::Result::Ok(#key) = #key_decoder.parse_next(input) else {
                        let __consumed = __key_offset_before - input.eof_offset();
                        input.reset(&checkpoint_inner);
                        if __consumed == 0 {
                            #clean_eof
                        }
                        return <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc).map(::tinyklv::decoder::Packet::Ready)
                    };

                    let #len = match #len_decoder.parse_next(input) {
                        ::core::result::Result::Ok(l) => l,
                        ::core::result::Result::Err(_) => {
                            #truncated
                        }
                    };
                }
            }
            // --------------------------------------------------
            // separate key and length, one-shot: any failure is truncation
            // --------------------------------------------------
            (
                DecodeFraming::Separate {
                    key_decoder,
                    len_decoder,
                    ..
                },
                LoopDriver::OneShot(_),
            ) => quote! {
                let ::core::result::Result::Ok(#key) = #key_decoder.parse_next(input) else {
                    #truncated
                };
                let ::core::result::Result::Ok(#len) = #len_decoder.parse_next(input) else {
                    #truncated
                };
            },
            // --------------------------------------------------
            // fused keylen header: a failure is truncation, and `__frame_offset` records
            // the input position at the frame start
            // --------------------------------------------------
            (DecodeFraming::Fused { keylen_decoder }, _) => {
                let on_fail = driver.on_parse_fail(ParseSite::Header);
                let header = quote_spanned! {helpers::call_site_at(keylen_decoder)=>
                    let __header: ::core::result::Result<(_, ::tinyklv::Framing), _> =
                        #keylen_decoder.parse_next(input);
                };
                quote! {
                    let __frame_offset = input.eof_offset();
                    #header
                    let (#key, __framing) = match __header {
                        ::core::result::Result::Ok(__frame) => __frame,
                        ::core::result::Result::Err(_) => {
                            #on_fail
                        }
                    };
                }
            }
        }
    }

    /// Emits the frame body: bound the value bytes and dispatch [`DecodeFraming::key`] to its
    /// field
    ///
    /// ```text
    /// // Separate
    /// #take(Subinput)
    /// match key { #arms(HeaderSliced) _ => (), }
    /// // Fused
    /// match __framing {
    ///     Sliced(__len) => { #sliced, as Separate with `__key` },
    ///     Inline => match __key { #arms(Inline) _ => { reset; break; } },
    /// }
    /// if input.eof_offset() == __frame_offset { #reject }
    /// ```
    ///
    /// A `size(exact = N)` field's frame length is checked in its arm before its value is decoded
    /// ([`ValueSrc::HeaderSliced`]), or, when the frame runs past the end of the input, before it
    /// is classed as truncation ([`LoopCtx::take`]); an `Inline` frame has no length to check
    ///
    /// An unknown key follows the rule stated on `tinyklv::Framing`: a `Sliced` one is dropped
    /// with its bytes, an `Inline` one ends the loop ([`LoopDriver::rewind_and_break`]), which is
    /// reported by [`DecodeFraming::unknown_key_breaks`]
    ///
    /// A fused frame whose header and value together consumed no input is rejected
    /// ([`LoopDriver::reject_label`]), since the loop would repeat it forever; a header that
    /// consumed nothing is fine when its value moved the cursor. The rule is stated on
    /// `tinyklv::Framing`
    ///
    /// # Returns
    ///
    /// The body statements
    pub(super) fn body(self) -> TokenStream {
        let key = self.framing.key();
        // --------------------------------------------------
        // the sliced form: take the value bytes, and dispatch the key to its field
        // --------------------------------------------------
        let take = self.take(TakeInto::Subinput);
        let arms = self.items_match(|_| ValueSrc::HeaderSliced);
        let sliced = quote! {
            #take
            match #key {
                #arms
                _ => (),
            }
        };
        // --------------------------------------------------
        // a separate header always slices; a fused one dispatches on the framing it produced
        // --------------------------------------------------
        match self.framing {
            DecodeFraming::Separate { .. } => sliced,
            DecodeFraming::Fused { .. } => {
                let len = self.framing.len();
                // --------------------------------------------------
                // the inline form: the field decoder reads from the main input, and an unknown
                // key rewinds and ends the loop
                // --------------------------------------------------
                let inline_arms = self.items_match(|_| ValueSrc::Inline);
                let end = LoopDriver::rewind_and_break();
                // --------------------------------------------------
                // a frame whose header and value consumed no input would repeat forever
                // --------------------------------------------------
                let empty = self.driver.reject_label(
                    &self.label(None, "its header and value together consumed no input"),
                );
                // --------------------------------------------------
                // dispatch on the framing, then reject a frame that moved nothing
                // --------------------------------------------------
                quote! {
                    match __framing {
                        ::tinyklv::Framing::Sliced(#len) => {
                            #sliced
                        }
                        ::tinyklv::Framing::Inline => match #key {
                            #inline_arms
                            _ => {
                                #end
                            }
                        },
                    }
                    if input.eof_offset() == __frame_offset {
                        #empty
                    }
                }
            }
        }
    }
}
