//! The frame header and frame body of a generated decode loop
//!
//! [`DecodeFraming`] is the codegen side of the container's framing mode. Each loop iteration reads
//! one frame header ([`DecodeFraming::header`]) and then decodes the value that follows it
//! ([`DecodeFraming::body`]):
//!
//! * [`DecodeFraming::Separate`] reads `key` then `len`, takes `len` bytes and dispatches every
//!   field against them ([`ValueSrc::HeaderSliced`])
//! * [`DecodeFraming::Fused`] reads `(key, Framing)` from the `keylen` codec and branches on the
//!   framing at run time: `Sliced(len)` is the separate body, `Inline` dispatches every field
//!   against the main stream ([`ValueSrc::Inline`])
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    helpers,
    key_match_gen::{ArmCtx, ValueSrc},
    loop_driver::{LoopCtx, LoopDriver, ParseSite, TakeInto},
};
use crate::ast::{attr::container::ContainerMode, types};

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
            ContainerMode::Separate { key, len } => Some(Self::Separate {
                key_decoder: key.dec.as_ref()?,
                len_decoder: len.dec.as_ref()?,
            }),
            ContainerMode::Fused(keylen) => Some(Self::Fused {
                keylen_decoder: keylen.dec.as_ref()?,
            }),
        }
    }

    /// Emits the frame header read, binding `key` and either `len` or `__framing`
    ///
    /// The separate header keeps its own failure rules. Streaming: a key decoder that fails
    /// without consuming a byte is truncation, one that consumed bytes is malformed and finalizes;
    /// any length failure is truncation. One-shot: any failure rewinds and stops. The fused header
    /// follows the rule stated on `tinyklv::Framing` ([`LoopDriver::on_parse_fail`] with
    /// [`ParseSite::Header`])
    ///
    /// The fused header result is bound with the type `Result<(_, Framing), _>`, located at the
    /// `keylen` decoder, so a decoder with the wrong return type is reported at the decoder
    ///
    /// # Arguments
    ///
    /// * `lp` - The loop exit policy and the accumulator type a malformed streaming key finalizes
    ///
    /// # Returns
    ///
    /// The header statements
    pub(super) fn header(self, lp: LoopCtx) -> TokenStream {
        let LoopCtx {
            driver, partial_ty, ..
        } = lp;
        let truncated = driver.truncated();
        match (self, driver) {
            (
                Self::Separate {
                    key_decoder,
                    len_decoder,
                },
                LoopDriver::Streaming,
            ) => {
                let clean_eof = driver.clean_eof();
                quote! {
                    let __key_offset_before = input.eof_offset();
                    let ::core::result::Result::Ok(key) = #key_decoder.parse_next(input) else {
                        let __consumed = __key_offset_before - input.eof_offset();
                        input.reset(&checkpoint_inner);
                        if __consumed == 0 {
                            #clean_eof
                        }
                        return <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc).map(::tinyklv::decoder::Packet::Ready)
                    };

                    let len = match #len_decoder.parse_next(input) {
                        ::core::result::Result::Ok(l) => l,
                        ::core::result::Result::Err(_) => {
                            #truncated
                        }
                    };
                }
            }
            (
                Self::Separate {
                    key_decoder,
                    len_decoder,
                },
                LoopDriver::OneShot,
            ) => quote! {
                let ::core::result::Result::Ok(key) = #key_decoder.parse_next(input) else {
                    #truncated
                };
                let ::core::result::Result::Ok(len) = #len_decoder.parse_next(input) else {
                    #truncated
                };
            },
            (Self::Fused { keylen_decoder }, _) => {
                let on_fail = driver.on_parse_fail(ParseSite::Header);
                let header = quote_spanned! {helpers::call_site_at(keylen_decoder)=>
                    let __header: ::core::result::Result<(_, ::tinyklv::Framing), _> =
                        #keylen_decoder.parse_next(input);
                };
                quote! {
                    #header
                    let ::core::result::Result::Ok((key, __framing)) = __header else {
                        #on_fail
                    };
                }
            }
        }
    }

    /// Emits the `debug` log line for the frame header just read
    ///
    /// # Returns
    ///
    /// A log of `key` with `len` (separate) or `__framing` (fused)
    pub(super) fn debug_line(self) -> TokenStream {
        let logger = crate::expand::logger::debug_logger();
        match self {
            Self::Separate { .. } => quote! { #logger ("key: {}, len: {}", key, len); },
            Self::Fused { .. } => quote! { #logger ("key: {}, framing: {:?}", key, __framing); },
        }
    }

    /// Emits the frame body: bound the value bytes and dispatch `key` to its field
    ///
    /// ```text
    /// // Separate
    /// #take
    /// match key { #arms(HeaderSliced) _ => (), }
    /// // Fused
    /// match __framing {
    ///     Sliced(len) => { #take match key { #arms(HeaderSliced) _ => (), } }
    ///     Inline => match key { #arms(Inline) _ => { reset; break; } },
    /// }
    /// ```
    ///
    /// An unknown key follows the rule stated on `tinyklv::Framing`: a `Sliced` one is dropped
    /// with its bytes, an `Inline` one ends the loop ([`LoopDriver::rewind_and_break`]), which is
    /// reported by [`Self::unknown_key_breaks`]
    ///
    /// # Arguments
    ///
    /// * `ctx` - The arm context, carrying the fields and the loop context the take is cut with
    ///
    /// # Returns
    ///
    /// The body statements
    pub(super) fn body(self, ctx: &ArmCtx) -> TokenStream {
        let take = ctx.lp.take(TakeInto::Subinput);
        let sliced_arms = ctx.items_match(|_| ValueSrc::HeaderSliced);
        let sliced = quote! {
            #take
            match key {
                #sliced_arms
                _ => (),
            }
        };
        match self {
            Self::Separate { .. } => sliced,
            Self::Fused { .. } => {
                let inline_arms = ctx.items_match(|_| ValueSrc::Inline);
                let end = LoopDriver::rewind_and_break();
                quote! {
                    match __framing {
                        ::tinyklv::Framing::Sliced(len) => {
                            #sliced
                        }
                        ::tinyklv::Framing::Inline => match key {
                            #inline_arms
                            _ => {
                                #end
                            }
                        },
                    }
                }
            }
        }
    }

    /// Whether [`Self::body`]'s unknown-key arm is a `break`, one of the reports
    /// [`DecodeLoopParts::breaks`](super::decode_partial_gen::DecodeLoopParts::breaks) ORs
    ///
    /// # Returns
    ///
    /// `true` for a fused frame, whose unknown `Inline` key ends the loop
    pub(super) fn unknown_key_breaks(self) -> bool {
        matches!(self, Self::Fused { .. })
    }
}
