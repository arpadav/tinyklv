//! The frame header and frame body of a generated decode loop
//!
//! [`DecodeFraming`] is the codegen side of the container's framing mode. Each loop iteration reads
//! one frame header ([`LoopCtx::header`]) and then decodes the value that follows it
//! ([`LoopCtx::body`]), both from the loop's own [`DecodeFraming`]:
//!
//! * [`DecodeFraming::Separate`] reads `key` then `len`, takes `len` bytes and dispatches every
//!   field against them. When a field is `size(implicit)` ([`LenRead::UnlessImplicit`]), a
//!   `match` on the key comes first: an implicit field's arm decodes its value with no length
//!   read, and every other key reads `len` and runs the rest of the frame
//!   ([`LoopCtx::after_key`]). With `unknown(..)` ([`LenRead::IfPrefixed`]) only the keys of
//!   the fields with a length prefix read `len`; the frame of a key no field declares is read
//!   whole by the `unknown` decoder ([`LoopCtx::unknown_from_input`]), and rejected when it moved
//!   the cursor nowhere
//! * [`DecodeFraming::Fused`] reads `(key, Framing)` from the `keylen` codec and branches on the
//!   framing at run time: `Sliced` is the separate body, `Inline` dispatches every field
//!   against the main stream ([`ValueSrc::Inline`]). With `unknown(..)` an unknown key's
//!   `Inline` value goes to the `unknown` decoder, and so does a `Sliced` one when the
//!   `#[klv(unknown)]` field keeps it ([`LoopCtx::kept_sliced_unknown_arm`]). A frame that moved
//!   the cursor nowhere is rejected after its value
//!
//! In either framing `break_on` runs right after the key ([`LoopCtx::break_on`]), a fused header's
//! length read with it, so a frame it ends or skips never reaches a field or the `unknown` decoder
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    ErrorKind, LoopCtx,
    key_match_gen::{DecodedFrom, KeyPattern, ValueSrc},
    loop_driver::{LoopDriver, ParseSite, TakeInto},
};
use crate::{
    ast::{
        attr::{MainContainer, UnknownField, UnknownFrames, container::ContainerMode},
        symbol, types,
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

/// The container's frame header codec(s) and `unknown(..)` decoder, carrying the borrowed
/// decoder tokens
///
/// Each variant borrows the decoders of its [`ContainerMode`], and the [`UnknownFrames`] decoder
/// where that mode's loop reads unknown frames
#[derive(Debug, Clone, Copy)]
pub(super) enum DecodeFraming<'a> {
    /// Separate key and length decoders; a known key's value with a length prefix is sliced via
    /// `take(len)`, while an `unknown(..)` decoder reads the main stream after the key
    Separate {
        /// Token expression decoding each frame's key
        key_decoder: &'a types::XcoderType,
        /// Token expression decoding each frame's length
        len_decoder: &'a types::XcoderType,
        /// Where the frame's length is read
        len_read: LenRead<'a>,
    },
    /// One decoder returning `(key, Framing)`, so the framing can depend on the key
    Fused {
        /// Token expression decoding each frame header into `(key, Framing)`
        keylen_decoder: &'a types::XcoderType,
        /// The decoder of an unknown key's value, with `unknown(..)`
        unknown: Option<UnknownDecoder<'a>>,
    },
}

/// Where a separate frame header reads the frame's length, chosen once per container
#[derive(Debug, Clone, Copy)]
pub(super) enum LenRead<'a> {
    /// Right after the key, before the `match` on it: the derive reads every frame's length
    /// ([`MainContainer::reads_every_len`])
    Hoisted,
    /// After the key, for every key but a `size(implicit)` field's, whose value has no length
    UnlessImplicit,
    /// After the key, only for the key of a field with a length prefix: the `unknown(..)` decoder
    /// reads every byte after a key no field declares, the frame's length included
    IfPrefixed(UnknownDecoder<'a>),
}

/// The [`UnknownFrames`] of a container that gets a decode impl, as its loop reads them
#[derive(Debug, Clone, Copy)]
pub(super) struct UnknownDecoder<'a> {
    /// Token expression of the `unknown(dec = ..)` decoder, called with the key of each unknown
    /// frame it reads
    pub(super) decoder: &'a types::XcoderType,

    /// The field a decoded frame is pushed into, or `None` when the frame is dropped
    pub(super) keep: Option<&'a UnknownField<'a>>,
}

/// [`DecodeFraming`] implementation
impl<'a> DecodeFraming<'a> {
    /// Borrows the decoder token(s) of a container's framing mode and its `unknown(..)`
    ///
    /// # Arguments
    ///
    /// * `input` - The container, whose validated framing mode and `unknown(..)` hold the
    ///   decoders and whose fields decide where a separate frame's length is read
    ///
    /// # Returns
    ///
    /// The framing, or `None` when a decoder is missing (`allow_unimplemented_decode`), in which
    /// case no decode impl is generated
    pub(super) fn of(input: &'a MainContainer<'a>) -> Option<Self> {
        let unknown = match &input.unknown {
            Some(UnknownFrames { codec, .. }) => Some(UnknownDecoder {
                decoder: codec.dec.as_ref()?,
                keep: input.kept(),
            }),
            None => None,
        };
        match &input.attrs.mode {
            ContainerMode::Separate { key, len, .. } => {
                let len_read = match unknown {
                    Some(unknown) => LenRead::IfPrefixed(unknown),
                    None if input.reads_every_len() => LenRead::Hoisted,
                    None => LenRead::UnlessImplicit,
                };
                Some(Self::Separate {
                    key_decoder: key.dec.as_ref()?,
                    len_decoder: len.dec.as_ref()?,
                    len_read,
                })
            }
            ContainerMode::Fused { keylen, .. } => Some(Self::Fused {
                keylen_decoder: keylen.dec.as_ref()?,
                unknown,
            }),
        }
    }

    /// The binding the frame header puts the decoded key in, which every later read of the key
    /// names
    ///
    /// Every local the generated code binds has an internal name, so a user `const` of the same
    /// name in scope is neither turned into a pattern nor reported against the derive
    ///
    /// # Returns
    ///
    /// `__key`
    pub(super) fn key() -> syn::Ident {
        quote::format_ident!("__key")
    }

    /// The binding the frame's value length is put in, which every `take` of the value and every
    /// `size(exact = N)` check reads, and which a `lengthed` decoder without `exact` is passed
    ///
    /// # Returns
    ///
    /// `__len`, internal as [`Self::key`] is
    pub(super) fn len() -> syn::Ident {
        quote::format_ident!("__len")
    }

    /// Emits the `debug` log line for the frame header just read
    ///
    /// # Returns
    ///
    /// A log of [`Self::key`] with [`Self::len`] (separate) or `__framing` (fused)
    pub(super) fn debug_line(self) -> TokenStream {
        let logger = crate::expand::logger::debug_logger();
        let key = Self::key();
        let len = Self::len();
        match self {
            Self::Separate { .. } => quote! { #logger ("key: {}, len: {}", #key, #len); },
            Self::Fused { .. } => quote! { #logger ("key: {}, framing: {:?}", #key, __framing); },
        }
    }
}

/// [`LoopCtx`] implementation of the frame header and body
impl LoopCtx<'_> {
    /// Emits the frame header read, binding [`DecodeFraming::key`], and `__framing` under a fused
    /// header
    ///
    /// The separate header reads the key alone, with its own failure rules. Streaming: a key
    /// decoder that fails at the end of the input, or without consuming a byte, is truncation, one
    /// that fails anywhere else is malformed and finalizes. One-shot: any failure rewinds and
    /// stops. [`Self::after_key`] reads the length. The fused header follows the
    /// rules stated on `tinyklv::Framing`: a failure is truncation
    /// ([`LoopDriver::on_parse_fail`] with [`ParseSite::Header`]). It also binds
    /// `__frame_offset`, the input position at the frame start, which [`Self::progress_check`]
    /// checks progress against
    ///
    /// The fused header result is bound with the type `Result<(_, Framing), _>`, located at the
    /// `keylen` decoder, so a decoder with the wrong return type is reported at the decoder. It is
    /// bound in a block of its own, which yields the key and the framing, so the result is dead
    /// once the header is read rather than stored in every frame
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
        let key = DecodeFraming::key();
        let truncated = driver.truncated();
        match (framing, driver) {
            // --------------------------------------------------
            // separate key, streaming: a key failing at the end of the input or without
            // consuming a byte is truncation; any other key failure finalizes
            // --------------------------------------------------
            (DecodeFraming::Separate { key_decoder, .. }, LoopDriver::Streaming) => quote! {
                let __key_offset_before = ::tinyklv::__export::winnow::stream::Stream::eof_offset(input);
                let ::core::result::Result::Ok(#key) = ::tinyklv::__export::winnow::Parser::parse_next(&mut #key_decoder, input) else {
                    if ::tinyklv::__export::winnow::stream::Stream::eof_offset(input) == 0
                        || ::tinyklv::__export::winnow::stream::Stream::eof_offset(input) == __key_offset_before
                    {
                        #truncated
                    }
                    ::tinyklv::__export::winnow::stream::Stream::reset(input, &__checkpoint_inner);
                    return <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc).map(::tinyklv::decoder::Packet::Ready)
                };
            },
            // --------------------------------------------------
            // separate key, one-shot: any failure is truncation
            // --------------------------------------------------
            (DecodeFraming::Separate { key_decoder, .. }, LoopDriver::OneShot(_)) => quote! {
                let ::core::result::Result::Ok(#key) = ::tinyklv::__export::winnow::Parser::parse_next(&mut #key_decoder, input) else {
                    #truncated
                };
            },
            // --------------------------------------------------
            // fused keylen header: a failure is truncation, and `__frame_offset` records
            // the input position at the frame start
            // --------------------------------------------------
            (DecodeFraming::Fused { keylen_decoder, .. }, _) => {
                let on_fail = driver.on_parse_fail(ParseSite::Header);
                let header = quote_spanned! {helpers::call_site_at(keylen_decoder)=>
                    let __header: ::core::result::Result<(_, ::tinyklv::Framing), _> =
                        ::tinyklv::__export::winnow::Parser::parse_next(&mut #keylen_decoder, input);
                };
                quote! {
                    let __frame_offset = ::tinyklv::__export::winnow::stream::Stream::eof_offset(input);
                    let (#key, __framing) = {
                        #header
                        match __header {
                            ::core::result::Result::Ok(__frame) => __frame,
                            ::core::result::Result::Err(_) => {
                                #on_fail
                            }
                        }
                    };
                }
            }
        }
    }

    /// Emits the read of a separate frame's length, binding [`DecodeFraming::len`]: any failure
    /// is truncation, except under [`LenRead::IfPrefixed`], where a failure follows the rule of a
    /// value read from the main stream, as the length the `unknown` decoder reads does
    /// ([`Self::value_err_arm`] with [`DecodedFrom::Input`]), naming the struct's item
    ///
    /// # Arguments
    ///
    /// * `len_decoder` - The separate frame's length decoder
    ///
    /// # Returns
    ///
    /// The `let` of the length, in the driver's spelling
    fn len_header(self, len_decoder: &types::XcoderType) -> TokenStream {
        let len = DecodeFraming::len();
        let truncated = self.driver.truncated();
        match (self.framing, self.driver) {
            (
                DecodeFraming::Separate {
                    len_read: LenRead::IfPrefixed(_),
                    ..
                },
                _,
            ) => {
                let on_err = self.value_err_arm(
                    DecodedFrom::Input,
                    helpers::call_site_at(len_decoder),
                    None,
                    &format!("`{}` decoder", symbol::LENGTH),
                );
                quote! {
                    let #len = match ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, input) {
                        ::core::result::Result::Ok(__len) => __len,
                        #on_err
                    };
                }
            }
            (_, LoopDriver::Streaming) => quote! {
                let #len = match ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, input) {
                    ::core::result::Result::Ok(__len) => __len,
                    ::core::result::Result::Err(_) => {
                        #truncated
                    }
                };
            },
            (_, LoopDriver::OneShot(_)) => quote! {
                let ::core::result::Result::Ok(#len) = ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, input) else {
                    #truncated
                };
            },
        }
    }

    /// Emits what follows the frame header and `break_on`: a separate frame's length read
    /// ([`Self::len_header`]) and `frame`, the rest of a frame that has a length (the `debug` log,
    /// the `deny_unknown_keys` gate and [`Self::body`])
    ///
    /// Under [`LenRead::Hoisted`] every key reads the length, then runs `frame`:
    ///
    /// ```text
    /// #len_header
    /// #frame
    /// ```
    ///
    /// Under [`LenRead::UnlessImplicit`] a `match` on the key comes first. A `size(implicit)`
    /// field's arm decodes its value with no length read ([`ValueSrc::implicit`]); every other
    /// key, an unknown one included, reads the length ([`Self::len_header`]) and then runs
    /// `frame`, so every prefixed field shares that one length read and the one `take` in
    /// [`Self::body`]:
    ///
    /// ```text
    /// match key {
    ///     #arms(Inline | Exact(N))
    ///     _ => {
    ///         #len_header
    ///         #frame
    ///     }
    /// }
    /// ```
    ///
    /// Under [`LenRead::IfPrefixed`] only the keys of the fields with a length prefix read the
    /// length and run `frame`, sharing one arm, left out when no field has a prefix. The arm
    /// carries the `clippy::manual_range_patterns` allow its pattern needs, if any
    /// ([`KeyPattern::allowed`]). Every other key is an unknown one, whose frame the `unknown`
    /// decoder reads from the main stream, its length included ([`Self::unknown_from_input`]),
    /// and which is rejected when it consumed no input ([`Self::progress_check`]). With no
    /// prefixed arm, and no packet the length frames, nothing reads the length, so the unknown arm
    /// type-checks its decoder in a read that never runs ([`Self::unread_len`]):
    ///
    /// ```text
    /// match key {
    ///     #arms(Inline | Exact(N))
    ///     K1 | K2 | .. => {
    ///         #len_header
    ///         #frame
    ///     }
    ///     _ => {
    ///         #unread_len         // no prefixed arm, no packet the length frames
    ///         #unknown_from_input
    ///         #progress_check
    ///     }
    /// }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `frame` - The rest of the frame, run once the length is read
    ///
    /// # Returns
    ///
    /// The length read and `frame`, the `match`, or `frame` itself when the header has no separate
    /// length
    pub(super) fn after_key(self, frame: TokenStream) -> TokenStream {
        let DecodeFraming::Separate {
            len_decoder,
            len_read,
            ..
        } = self.framing
        else {
            return frame;
        };
        // --------------------------------------------------
        // the length read and the rest of the frame: for every key, or, in the arms after the
        // implicit fields', for every other key or for the prefixed fields' keys only
        // --------------------------------------------------
        let rest = match len_read {
            LenRead::Hoisted => {
                let len = self.len_header(len_decoder);
                return quote! {
                    #len
                    #frame
                };
            }
            LenRead::UnlessImplicit => {
                let len = self.len_header(len_decoder);
                quote! {
                    _ => {
                        #len
                        #frame
                    }
                }
            }
            LenRead::IfPrefixed(unknown) => {
                // --------------------------------------------------
                // the keys `sliced_body` dispatches, so each reaches the one length read
                // --------------------------------------------------
                let pattern: KeyPattern = self
                    .input
                    .klv_keys()
                    .filter(|key| ValueSrc::implicit(key.codec.size.as_ref()).is_none())
                    .map(|key| key.key)
                    .collect();
                let prefixed = (!pattern.is_empty()).then(|| {
                    let len = self.len_header(len_decoder);
                    pattern.allowed(quote! {
                        #pattern => {
                            #len
                            #frame
                        }
                    })
                });
                // --------------------------------------------------
                // every other key's frame, read whole by the `unknown` decoder, then rejected when
                // it moved the cursor nowhere; a length no key and no packet reads is still
                // type-checked
                // --------------------------------------------------
                let unread = (prefixed.is_none() && !self.input.attrs.mode.len_frames_packet())
                    .then(|| Self::unread_len(len_decoder));
                let unknown = self.unknown_from_input(unknown);
                let progress = self.progress_check();
                quote! {
                    #prefixed
                    _ => {
                        #unread
                        #unknown
                        #progress
                    }
                }
            }
        };
        let key = DecodeFraming::key();
        let implicit_arms = self.items_match(ValueSrc::implicit);
        quote! {
            match #key {
                #implicit_arms
                #rest
            }
        }
    }

    /// Emits a length read that never runs, for a separate frame header whose length neither a
    /// key nor a packet's seek reads: the length decoder is type-checked, a `usize` as every
    /// frame length is, located at the decoder so that one of another type is reported there, and
    /// is not left unused
    ///
    /// # Arguments
    ///
    /// * `len_decoder` - The separate frame's length decoder
    ///
    /// # Returns
    ///
    /// `if false { let _: Result<usize, _> = Parser::parse_next(&mut #len_decoder, input); }`
    fn unread_len(len_decoder: &types::XcoderType) -> TokenStream {
        quote_spanned! {helpers::call_site_at(len_decoder)=>
            if false {
                let _: ::core::result::Result<::core::primitive::usize, _> =
                    ::tinyklv::__export::winnow::Parser::parse_next(&mut #len_decoder, input);
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
    /// // Separate, every field `size(implicit)`
    /// #take(Discard)
    /// // Fused
    /// match __framing {
    ///     Sliced(__len) => { #sliced, as Separate with `__key` },
    ///     Inline => match __key { #arms(Inline) _ => { reset; break; } },
    ///     _ => unreachable!(),
    /// }
    /// // Fused, `unknown(..)`: the `_` arms read the unknown key's value
    /// Sliced(__len) => { #sliced, its `_` arm #kept_sliced_unknown_arm when the field keeps it },
    /// Inline => match __key { #arms(Inline) _ => #unknown_from_input, },
    /// ```
    ///
    /// Only a field with a length prefix gets a sliced arm; a `size(implicit)` one gets its arm in
    /// [`Self::after_key`]
    ///
    /// A `size(exact = N)` field's frame length is checked in its arm before its value is decoded
    /// ([`ValueSrc::HeaderSliced`]), or, when the frame runs past the end of the input, before it
    /// is classed as truncation ([`LoopCtx::take`]); an `Inline` frame has no length to check
    ///
    /// An unknown key follows the rule stated on `tinyklv::Framing`: a `Sliced` one is dropped
    /// with its bytes, an `Inline` one ends the loop ([`LoopDriver::rewind_and_break`]). With
    /// `unknown(..)` the `unknown` decoder reads it instead: an `Inline` one's main stream
    /// ([`Self::unknown_from_input`]), and a `Sliced` one's bounded value when the
    /// `#[klv(unknown)]` field keeps it ([`Self::kept_sliced_unknown_arm`]). A separate frame's
    /// `_ => ()` drops an unknown key's bytes; with `unknown(..)` no unknown key reaches it
    /// ([`LenRead::IfPrefixed`])
    ///
    /// A fused frame whose header and value together consumed no input is rejected after the
    /// body, by the check [`Self::frame_loop`] puts after every fused frame
    /// ([`Self::progress_check`])
    ///
    /// # Returns
    ///
    /// The body statements
    pub(super) fn body(self) -> TokenStream {
        // --------------------------------------------------
        // a separate header always slices; a fused one dispatches on the framing it produced
        // --------------------------------------------------
        let DecodeFraming::Fused { unknown, .. } = self.framing else {
            return self.sliced_body(None);
        };
        let key = DecodeFraming::key();
        let len = DecodeFraming::len();
        let sliced = self.sliced_body(unknown);
        // --------------------------------------------------
        // the inline form: the field decoder reads from the main input, and an unknown key goes
        // to the `unknown` decoder on the main input, or without one rewinds and ends the loop
        // --------------------------------------------------
        let inline_arms = self.items_match(|_| Some(ValueSrc::Inline));
        let inline_unknown = if let Some(unknown) = unknown {
            let unknown = self.unknown_from_input(unknown);
            quote! { _ => #unknown, }
        } else {
            let end = LoopDriver::rewind_and_break();
            quote! {
                _ => {
                    #end
                }
            }
        };
        // --------------------------------------------------
        // dispatch on the framing
        // --------------------------------------------------
        quote! {
            match __framing {
                ::tinyklv::Framing::Sliced(#len) => {
                    #sliced
                }
                ::tinyklv::Framing::Inline => match #key {
                    #inline_arms
                    #inline_unknown
                },
                // `Framing` is `#[non_exhaustive]`, and this derive matches each of its variants
                _ => ::core::unreachable!(),
            }
        }
    }

    /// Emits the rejection of a frame whose header and value together consumed no input, since
    /// the loop would repeat it forever
    ///
    /// ```text
    /// // Fused
    /// if Stream::eof_offset(input) == __frame_offset {
    ///     #reject
    /// }
    /// // Separate
    /// if Offset::offset_from(&*input, &__checkpoint_inner) == 0 {
    ///     #reject
    /// }
    /// ```
    ///
    /// It runs after every fused frame ([`Self::frame_loop`]), against the frame start its header
    /// binds ([`Self::header`]), and after a separate frame's unknown value
    /// ([`Self::after_key`]), against the frame start `__checkpoint_inner`, so a separate frame's
    /// header binds nothing for it; in either framing, it also runs before a `break_on` fn's
    /// `Skip` repeats the loop ([`Self::break_on`]). Each framing's form decodes it in fewer
    /// instructions than the other's would. The cursor is still at the frame start when it
    /// rejects. A header that consumed nothing is fine when its value moved the cursor. The rule
    /// is stated on `tinyklv::Framing`
    ///
    /// # Returns
    ///
    /// The check, rejecting through [`LoopDriver::reject_label`]
    pub(super) fn progress_check(self) -> TokenStream {
        let empty = self.driver.reject_label(
            &self.label(None, "its header and value together consumed no input"),
            ErrorKind::NoProgress,
        );
        let consumed_nothing = match self.framing {
            DecodeFraming::Fused { .. } => quote! {
                ::tinyklv::__export::winnow::stream::Stream::eof_offset(input) == __frame_offset
            },
            DecodeFraming::Separate { .. } => quote! {
                ::tinyklv::__export::winnow::stream::Offset::offset_from(&*input, &__checkpoint_inner) == 0
            },
        };
        quote! {
            if #consumed_nothing {
                #empty
            }
        }
    }

    /// Emits the sliced form of the frame body: take the value bytes, and dispatch
    /// [`DecodeFraming::key`] to its field
    ///
    /// ```text
    /// #take(Subinput)
    /// match key { #arms(HeaderSliced) _ => (), }
    /// // the `#[klv(unknown)]` field keeps the unknown frames
    /// #take(Subinput)
    /// match key { #arms(HeaderSliced) #kept_sliced_unknown_arm }
    /// // no field with a length prefix, none keeping the unknown frames
    /// #take(Discard)
    /// ```
    ///
    /// Under the container's `strict` the arms are [`ValueSrc::StrictSliced`]
    ///
    /// # Arguments
    ///
    /// * `unknown` - The decoder of an unknown key's value, with `unknown(..)`; an unknown key's
    ///   bytes go with the `take` unless the `#[klv(unknown)]` field keeps the frame
    ///
    /// # Returns
    ///
    /// The `take` and the `match`, or the `take` alone when nothing reads the bytes
    fn sliced_body(self, unknown: Option<UnknownDecoder>) -> TokenStream {
        let key = DecodeFraming::key();
        // --------------------------------------------------
        // take the value bytes, and dispatch the key to its field or, when the field keeps the
        // frame, the `unknown` decoder; with neither to read them, the bytes are dropped
        // --------------------------------------------------
        let arms = self.items_match(|size| ValueSrc::header_sliced(self.input, size));
        let unknown = match unknown {
            Some(UnknownDecoder {
                decoder,
                keep: Some(field),
            }) => self.kept_sliced_unknown_arm(decoder, field),
            Some(UnknownDecoder { keep: None, .. }) | None if arms.is_empty() => {
                return self.take(TakeInto::Discard);
            }
            Some(UnknownDecoder { keep: None, .. }) | None => quote! { _ => (), },
        };
        let take = self.take(TakeInto::Subinput);
        quote! {
            #take
            match #key {
                #arms
                #unknown
            }
        }
    }
}
