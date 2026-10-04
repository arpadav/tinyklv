//! The exit policy of a generated decode loop
//!
//! Both decode entry points run the same per-frame loop and differ only in how an iteration leaves
//! it. [`LoopDriver`] owns the exits every framing shares, so each is spelled once per driver. The
//! classic separate streaming header is the exception: it keeps its own malformed-key finalize so
//! its output stays byte-identical (see `DecodeFraming::header`):
//!
//! * [`LoopDriver::Streaming`] - `resume_partial`: truncation rewinds and returns
//!   `Packet::NeedMore`, an error returns a bare `&'static str` label, which names the field for a
//!   malformed `Inline` value
//! * [`LoopDriver::OneShot`] - `decode_value`: truncation rewinds and `break`s to the finalize
//!   tail, an error returns a `ContextError` labelled at the input position, except a malformed
//!   `Inline` value, which returns the value decoder's own error with the field added as context
//!   ([`LoopDriver::reject_value`])
//!
//! The rules that decide truncation against malformed are stated once, on `tinyklv::Framing`
//! and `tinyklv::Framing::Inline`
//!
//! The exits that slice or finalize also name the stream and accumulator types, so they live on
//! [`LoopCtx`], which pairs a driver with those types
//!
//! The emitted tokens assume the loop's bindings: `input`, `checkpoint` (fn entry),
//! `checkpoint_inner` (frame start), `__acc` (the partial accumulator), `len` (the frame's value
//! length, wherever a `take` is cut) and, inside a failed `Inline` value arm, `__err` (the value
//! decoder's error)
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// Which decode entry point a loop is emitted for, and therefore how it exits
#[derive(Debug, Clone, Copy)]
pub(super) enum LoopDriver {
    /// `ResumePartial::resume_partial`: resumable across `feed` chunks via `Packet::NeedMore`
    Streaming,
    /// `DecodeValue::decode_value`: one linear pass over a complete value body
    OneShot,
}

/// What [`LoopCtx::take`] does with the bytes it cuts off the input
#[derive(Debug, Clone, Copy)]
pub(super) enum TakeInto {
    /// Bind them as `subinput`, the bounded slice the `HeaderSliced` arms decode from
    Subinput,
    /// Drop them (`break_on` returned `Skip`)
    Discard,
}

/// Where a parse failed, for [`LoopDriver::on_parse_fail`]
#[derive(Debug, Clone, Copy)]
pub(super) enum ParseSite<'a> {
    /// The fused `keylen` frame header
    Header,
    /// An `Inline` value read straight from the main stream
    Value {
        /// The `&'static str` noun phrase naming the field, which a malformed value is rejected
        /// with
        label: &'a TokenStream,
    },
}

/// A [`LoopDriver`] together with the types its slicing and finalizing exits name
#[derive(Debug, Clone, Copy)]
pub(super) struct LoopCtx<'a> {
    /// The loop exit policy
    pub(super) driver: LoopDriver,

    /// The stream type the impl is parameterised over, which `take` is instantiated over
    pub(super) stream: &'a syn::Type,

    /// The partial accumulator type, `#partial_name #ty_generics`, whose `Partial::finalize` the
    /// finalizing exits call
    pub(super) partial_ty: &'a TokenStream,
}

/// [`LoopDriver`] implementation
impl LoopDriver {
    /// Emits the exit taken when the input is empty at the top of an iteration
    ///
    /// Nothing was consumed, so there is nothing to rewind. The classic separate streaming header
    /// also takes it, after its rewind, when the key decoder fails without consuming a byte
    ///
    /// Whether it is a `break` is reported by [`Self::clean_eof_breaks`]
    ///
    /// # Returns
    ///
    /// `return Ok(NeedMore(__acc));` (streaming) or `break;` (one-shot)
    pub(super) fn clean_eof(self) -> TokenStream {
        match self {
            Self::Streaming => quote! {
                return ::core::result::Result::Ok(
                    ::tinyklv::decoder::Packet::NeedMore(__acc)
                );
            },
            Self::OneShot => quote! { break; },
        }
    }

    /// Whether [`Self::clean_eof`] is a `break`, one of the reports
    /// [`DecodeLoopParts::breaks`](super::decode_partial_gen::DecodeLoopParts::breaks) ORs
    ///
    /// Every loop takes [`Self::clean_eof`] at the top of an iteration, so this also covers the
    /// other exits that `break` only under one-shot ([`Self::truncated`] and every exit built on
    /// it)
    ///
    /// # Returns
    ///
    /// `true` for one-shot
    pub(super) fn clean_eof_breaks(self) -> bool {
        matches!(self, Self::OneShot)
    }

    /// Emits the exit taken when the current frame runs past the end of the input
    ///
    /// Rewinds to the frame start first, so the caller sees the cursor where the frame began:
    /// streaming resumes from there once more bytes arrive, one-shot finalizes without the frame
    ///
    /// One-shot has no more bytes coming, yet it still rewinds the truncated frame before
    /// finalize. That leaves the cursor exactly where the streaming path's `reset` + `NeedMore`
    /// would, so `Vec<T>::decode_value` and other cursor-reading callers observe identical residual
    /// input
    ///
    /// # Returns
    ///
    /// The reset followed by `return Ok(NeedMore(__acc));` (streaming) or `break;` (one-shot)
    pub(super) fn truncated(self) -> TokenStream {
        match self {
            Self::Streaming => {
                let exit = self.clean_eof();
                quote! {
                    input.reset(&checkpoint_inner);
                    #exit
                }
            }
            Self::OneShot => Self::rewind_and_break(),
        }
    }

    /// Emits the exit that rewinds to the frame start and ends the loop, for either driver
    ///
    /// A streaming loop reaches it only through a fused frame's unknown `Inline` key, reported by
    /// [`DecodeFraming::unknown_key_breaks`](super::frame_gen::DecodeFraming::unknown_key_breaks);
    /// a one-shot loop already breaks at [`Self::clean_eof`]
    ///
    /// # Returns
    ///
    /// `input.reset(&checkpoint_inner); break;`
    pub(super) fn rewind_and_break() -> TokenStream {
        quote! {
            input.reset(&checkpoint_inner);
            break;
        }
    }

    /// Emits a fatal return carrying `msg`, with no finalize
    ///
    /// This is the `break_on` `Abort` arm too. `Abort` is documented to *guarantee* an `Err` with
    /// no partially-decoded value surfaced, so this exit honors that contract rather than
    /// finalizing what was accumulated; both drivers return `Err` without finalizing
    ///
    /// # Arguments
    ///
    /// * `msg` - An expression evaluating to the `&'static str` label
    ///
    /// # Returns
    ///
    /// `return Err(#msg);` (streaming) or `return Err(labeled_error(input, &checkpoint, #msg));`
    /// (one-shot)
    pub(super) fn reject(self, msg: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => quote! { return ::core::result::Result::Err(#msg); },
            Self::OneShot => quote! {
                return ::core::result::Result::Err(::tinyklv::__export::labeled_error(
                    input,
                    &checkpoint,
                    #msg,
                ));
            },
        }
    }

    /// Emits the handling of a failed fused-header or `Inline` value parse
    ///
    /// Encodes the rules stated on `tinyklv::Framing` (header) and `tinyklv::Framing::Inline`
    /// (value):
    ///
    /// * [`ParseSite::Header`] - every failure is truncation ([`Self::truncated`])
    /// * [`ParseSite::Value`] - a failure at the end of the input is truncation, one anywhere else
    ///   is malformed ([`Self::reject_value`]); the end-of-input test runs before any rewind
    ///
    /// # Arguments
    ///
    /// * `site` - The failed parse; a [`ParseSite::Value`] carries its rejection label
    ///
    /// # Returns
    ///
    /// The statements for the parse's `Err` branch; every path leaves the iteration
    pub(super) fn on_parse_fail(self, site: ParseSite) -> TokenStream {
        let truncated = self.truncated();
        match site {
            ParseSite::Header => truncated,
            ParseSite::Value { label } => {
                let malformed = self.reject_value(label);
                quote! {
                    if input.eof_offset() == 0 {
                        #truncated
                    }
                    #malformed
                }
            }
        }
    }

    /// Emits the fatal return for a malformed `Inline` value, with no finalize
    ///
    /// Streaming always returns `` invalid `<Struct>::<field>` value (..) ``, prefixing `invalid `
    /// itself. One-shot returns the value decoder's own error with `label` added as a context.
    /// winnow's `ContextError` display shows only the first label, after `invalid `, so `label` is
    /// shown first only when the decoder's error has no label of its own; otherwise it stays in
    /// the context chain
    ///
    /// # Arguments
    ///
    /// * `label` - An expression evaluating to the `&'static str` noun phrase naming the field
    ///
    /// # Returns
    ///
    /// `return Err(concat!("invalid ", #label));` (streaming) or `__err` with `label` added as a
    /// `StrContext::Label` (one-shot)
    fn reject_value(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => quote! {
                return ::core::result::Result::Err(concat!("invalid ", #label));
            },
            Self::OneShot => quote! {
                return ::core::result::Result::Err(
                    ::tinyklv::__export::winnow::error::AddContext::add_context(
                        __err,
                        input,
                        &checkpoint_inner,
                        ::tinyklv::__export::winnow::error::StrContext::Label(#label),
                    ),
                );
            },
        }
    }
}

/// [`LoopCtx`] implementation
impl LoopCtx<'_> {
    /// Emits a pre-checked `take` of the frame's `len` bytes off the main input
    ///
    /// The pre-check classes a short frame as truncation before `take` runs, so the `take` failure
    /// arm of [`TakeInto::Subinput`] is unreachable in practice; it finalizes (streaming) or
    /// rewinds and stops (one-shot)
    ///
    /// # Arguments
    ///
    /// * `into` - Whether the bytes are bound as `subinput` or dropped
    ///
    /// # Returns
    ///
    /// The pre-check followed by the `take` statement
    pub(super) fn take(self, into: TakeInto) -> TokenStream {
        let Self { driver, stream, .. } = self;
        let truncated = driver.truncated();
        let take = quote! {
            ::tinyklv::__export::winnow::token::take::<
                usize,
                #stream,
                ::tinyklv::__export::winnow::error::ContextError,
            >(len).parse_next(input)
        };
        let bind = match into {
            TakeInto::Discard => quote! { let _ = #take; },
            TakeInto::Subinput => {
                let fail_arm = match driver {
                    LoopDriver::Streaming => {
                        let finish = self.finish();
                        quote! { return #finish, }
                    }
                    LoopDriver::OneShot => quote! { { #truncated } },
                };
                quote! {
                    let mut subinput: <#stream as ::tinyklv::__export::winnow::stream::Stream>::Slice = match #take {
                        ::core::result::Result::Ok(s) => s,
                        ::core::result::Result::Err(_) => #fail_arm
                    };
                }
            }
        };
        quote! {
            if input.eof_offset() < len {
                #truncated
            }
            #bind
        }
    }

    /// Emits the `match` on a `break_on` decision, when `break_on` is set
    ///
    /// `Skip` discards the value via [`Self::take`] and continues, `Done` ends the loop right
    /// after the header (no rewind, the header stays consumed), `Abort` is a
    /// [`LoopDriver::reject`] with the user's message, and `Proceed` (and any future variant) falls
    /// through to the value dispatch
    ///
    /// The `Done` arm is a `break`, so a `Some` is the report
    /// [`DecodeLoopParts::breaks`](super::decode_partial_gen::DecodeLoopParts::breaks) ORs
    ///
    /// # Arguments
    ///
    /// * `decision` - The per-iteration `BreakType` scrutinee, present iff `break_on` was set
    ///
    /// # Returns
    ///
    /// The break-handling `match`, or `None` when `break_on` is unset
    pub(super) fn break_match(self, decision: Option<&TokenStream>) -> Option<TokenStream> {
        let decision = decision?;
        let skip = self.take(TakeInto::Discard);
        let abort = self.driver.reject(&quote! { __msg });
        Some(quote! {
            match #decision {
                ::tinyklv::BreakType::Proceed => (),
                ::tinyklv::BreakType::Skip => {
                    #skip
                    continue;
                }
                ::tinyklv::BreakType::Done => break,
                ::tinyklv::BreakType::Abort(__msg) => {
                    #abort
                }
                // future `BreakType` variants behave as `Proceed`
                _ => (),
            }
        })
    }

    /// Emits the tail expression that finalizes `__acc` once the loop ends
    ///
    /// # Returns
    ///
    /// `Ok(Ready(v))` / `Err(label)` (streaming) or the struct with a missing-field label mapped
    /// to a `ContextError` (one-shot)
    pub(super) fn finish(self) -> TokenStream {
        let partial_ty = self.partial_ty;
        match self.driver {
            LoopDriver::Streaming => quote! {
                match <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc) {
                    ::core::result::Result::Ok(v) => ::core::result::Result::Ok(
                        ::tinyklv::decoder::Packet::Ready(v)
                    ),
                    ::core::result::Result::Err(label) => ::core::result::Result::Err(label),
                }
            },
            LoopDriver::OneShot => quote! {
                <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc)
                    .map_err(|label| ::tinyklv::__export::labeled_error(input, &checkpoint, label))
            },
        }
    }
}
