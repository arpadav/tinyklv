//! The exit policy of a generated decode loop
//!
//! Both decode entry points run the same per-frame loop and differ only in how an iteration leaves
//! it. [`LoopDriver`] owns the exits every framing shares, so each is spelled once per driver. The
//! classic separate streaming header is the exception: it keeps its own malformed-key finalize so
//! its output stays byte-identical (see `LoopCtx::header`):
//!
//! * [`LoopDriver::Streaming`] - `resume_partial`: truncation rewinds and returns
//!   `Packet::NeedMore`, an error returns a bare `&'static str` message, which reads
//!   `invalid <label>` for a rejection the derive labels ([`LoopDriver::reject_label`])
//! * [`LoopDriver::OneShot`] - `decode_value`: truncation rewinds and `break`s to the finalize
//!   tail, an error leaves the fn with a `ContextError` labelled at the input position, except a
//!   malformed value read from the main stream or kept from a `Sliced` unknown frame, whose error
//!   names the field (or an unknown frame) first, then the value decoder's own error
//!   ([`LoopDriver::reject_value`]). Its
//!   [`CursorSite`] says which cursor the loop advances, and so how an error leaves the fn
//!
//! The rules that decide truncation against malformed are stated once, on `tinyklv::Framing`
//! and `tinyklv::Framing::Inline`
//!
//! The exits that slice or finalize also name the stream and accumulator types, so they live on
//! [`LoopCtx`], which pairs a driver with those types
//!
//! The emitted tokens assume the loop's bindings: `input`, `checkpoint` (fn entry),
//! `checkpoint_inner` (frame start), `__acc` (the partial accumulator), the frame's value length
//! (`DecodeFraming::len`, wherever a `take` is cut or a `size(exact = N)` check reads it),
//! inside a failed main-stream or kept `Sliced` unknown value arm, `__err` (the value decoder's
//! error), inside the `Abort` arm of a field's `break_on`, `__msg` (its message) and, in a
//! one-shot loop over a [`CursorSite::Local`] cursor, `__exact_reject` (the held
//! `size(exact = N)` rejection)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    LoopCtx,
    framing::{DecodeFraming, LenRead},
};
use crate::ast::attr::{MainContainer, container::BreakOnSpec};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

/// Which decode entry point a loop is emitted for, and therefore how it exits
#[derive(Debug, Clone, Copy)]
pub(super) enum LoopDriver {
    /// `ResumePartial::resume_partial`: resumable across `feed` chunks via `Packet::NeedMore`
    Streaming,
    /// `DecodeValue::decode_value`: one linear pass over a complete value body, advancing the
    /// cursor its [`CursorSite`] names
    OneShot(CursorSite),
}

/// Which cursor a one-shot loop advances, chosen once per container from its fields and stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CursorSite {
    /// The caller's, `input` itself: an error `return`s
    Caller,
    /// A local copy of the caller's, inside the block labelled `'__tinyklv_decode`: an error
    /// `break`s out of the block, and the copy is written back to the caller's after it. A
    /// `size(exact = N)` rejection is held in `__exact_reject` until the loop ends
    Local,
}

/// [`CursorSite`] implementation
impl CursorSite {
    /// Picks which cursor a container's one-shot loop advances
    ///
    /// # Arguments
    ///
    /// * `input` - The container, whose fields decide it
    /// * `stream` - The stream type the loop decodes, which the local copy is cloned from
    ///
    /// # Returns
    ///
    /// [`Self::Local`] when a field's frame length is checked against its `size(exact = N)`
    /// ([`SizeSpec::len_check`](crate::ast::attr::size::SizeSpec::len_check)) and the stream is
    /// `&[u8]`, else [`Self::Caller`]: a container without such a field decodes over the caller's
    /// cursor in fewer instructions, and any other stream need not be `Clone`
    pub(super) fn of(input: &MainContainer, stream: &syn::Type) -> Self {
        let exact = input
            .klv_keys()
            .filter_map(|key| key.codec.size.as_ref())
            .any(|size| size.len_check().is_some());
        if exact && *stream == super::u8_slice() {
            Self::Local
        } else {
            Self::Caller
        }
    }

    /// Emits the keyword an error leaves the decode fn with, its value following
    ///
    /// # Returns
    ///
    /// `return` (caller) or `break '__tinyklv_decode` (local)
    fn error_exit(self) -> TokenStream {
        match self {
            Self::Caller => quote! { return },
            Self::Local => quote! { break '__tinyklv_decode },
        }
    }
}

/// How many bytes [`LoopCtx::take`] cuts off the input, and what it does with them
#[derive(Debug, Clone, Copy)]
pub(super) enum TakeInto {
    /// Bind the frame's length in bytes as `subinput`, the bounded slice a known-key arm, or the
    /// `unknown` decoder of a kept `Sliced` frame, decodes from; a frame running past the end of
    /// the input first meets the `size(exact = N)` checks ([`LoopCtx::exact_guards`])
    Subinput,
    /// Bind `N` bytes as `subinput` for a `size(implicit, exact = N)` value, whose frame has no
    /// length to check
    Exact(usize),
    /// Drop the frame's length in bytes (`break_on` returned `Skip`, or no field reads them)
    Discard,
}

/// Where a parse failed, for [`LoopDriver::on_parse_fail`]
#[derive(Debug, Clone, Copy)]
pub(super) enum ParseSite<'a> {
    /// The fused `keylen` frame header
    Header,
    /// A value read straight from the main stream: an `Inline` one, or an unknown key's
    Value {
        /// Names the field, or for an unknown key with no `#[klv(unknown)]` field the struct's
        /// frame, which a malformed value is rejected with (a [`LoopDriver::reject_label`] label)
        label: &'a TokenStream,
    },
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
            Self::OneShot(_) => quote! { break; },
        }
    }

    /// Whether [`Self::clean_eof`] is a `break`, one of the reports
    /// [`DecodeCtx::breaks`](super::DecodeCtx::breaks) ORs
    ///
    /// Every loop takes [`Self::clean_eof`] at the top of an iteration, so this also covers the
    /// other exits that `break` only under one-shot ([`Self::truncated`] and every exit built on
    /// it)
    ///
    /// # Returns
    ///
    /// `true` for one-shot
    pub(super) fn clean_eof_breaks(self) -> bool {
        matches!(self, Self::OneShot(_))
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
    /// The reset followed by [`Self::clean_eof`]: `return Ok(NeedMore(__acc));` (streaming) or
    /// `break;` (one-shot)
    pub(super) fn truncated(self) -> TokenStream {
        let rewind = Self::rewind();
        let exit = self.clean_eof();
        quote! {
            #rewind
            #exit
        }
    }

    /// Emits the exit that rewinds to the frame start and ends the loop, for either driver
    ///
    /// A streaming loop reaches it only through a fused frame's unknown `Inline` key, reported by
    /// [`DecodeFraming::unknown_key_breaks`](super::framing::DecodeFraming::unknown_key_breaks);
    /// a one-shot loop already breaks at [`Self::clean_eof`]
    ///
    /// # Returns
    ///
    /// `input.reset(&checkpoint_inner); break;`
    pub(super) fn rewind_and_break() -> TokenStream {
        let rewind = Self::rewind();
        quote! {
            #rewind
            break;
        }
    }

    /// Emits the rewind to the frame start
    ///
    /// # Returns
    ///
    /// `input.reset(&checkpoint_inner);`
    fn rewind() -> TokenStream {
        quote! { input.reset(&checkpoint_inner); }
    }

    /// Emits a fatal exit carrying `msg`, with no finalize
    ///
    /// This is the container `break_on`'s `Abort` arm too. `Abort` is documented to *guarantee*
    /// an `Err` with no partially-decoded value surfaced, so this exit honors that contract rather
    /// than finalizing what was accumulated; both drivers leave with `Err` without finalizing
    ///
    /// # Arguments
    ///
    /// * `msg` - An expression evaluating to the `&'static str` message, returned as is
    ///   (streaming) or as the error's label (one-shot)
    ///
    /// # Returns
    ///
    /// `return Err(#msg);` (streaming) or `#exit Err(labeled_error(input, &checkpoint, #msg));`,
    /// `#exit` the [`CursorSite`]'s error exit (one-shot)
    pub(super) fn reject(self, msg: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => quote! { return ::core::result::Result::Err(#msg); },
            Self::OneShot(cursor) => {
                let exit = cursor.error_exit();
                quote! {
                    #exit ::core::result::Result::Err(::tinyklv::__export::labeled_error(
                        input,
                        &checkpoint,
                        #msg,
                    ));
                }
            }
        }
    }

    /// Emits a fatal exit naming `label`, with no finalize ([`Self::reject`])
    ///
    /// Streaming prefixes the `invalid ` that a one-shot `ContextError`'s display prints before
    /// its label, so both read `invalid <label>`
    ///
    /// # Arguments
    ///
    /// * `label` - A `::core::concat!(..)` of string literals naming what was rejected, so that
    ///   streaming can splice it into its own `concat!`
    ///
    /// # Returns
    ///
    /// `return Err(concat!("invalid ", #label));` (streaming) or
    /// `#exit Err(labeled_error(input, &checkpoint, #label));` (one-shot, see [`Self::reject`])
    pub(super) fn reject_label(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => self.reject(&quote! { ::core::concat!("invalid ", #label) }),
            Self::OneShot(_) => self.reject(label),
        }
    }

    /// Emits the fatal exit for a frame whose length is not its field's `size(exact = N)`, with
    /// no finalize
    ///
    /// A one-shot loop over a [`CursorSite::Local`] cursor holds `label` in `__exact_reject` for
    /// [`LoopCtx::finish`] to return, so every such rejection shares its one error exit, and
    /// leaves the loop; any other loop leaves through [`Self::reject_label`]
    ///
    /// # Arguments
    ///
    /// * `label` - Names the field and its `N` (a [`Self::reject_label`] label)
    ///
    /// # Returns
    ///
    /// `__exact_reject = Some(#label); break;` (one-shot, local cursor) or the
    /// [`Self::reject_label`] exit
    pub(super) fn reject_exact(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::OneShot(CursorSite::Local) => quote! {
                __exact_reject = ::core::option::Option::Some(#label);
                break;
            },
            Self::Streaming | Self::OneShot(CursorSite::Caller) => self.reject_label(label),
        }
    }

    /// Emits the `unknown(..)` decoder's read of the main stream, `read` given the cursor it
    /// advances
    ///
    /// A one-shot loop over a [`CursorSite::Local`] cursor hands `read` a copy of that cursor and
    /// writes the copy back after it, so a decoder that is not inlined takes the copy's address,
    /// not the loop cursor's, which stays in a register; any other loop hands it `input`
    ///
    /// ```text
    /// // one-shot, local cursor
    /// { let mut __input = *input; let __read = #read(&mut __input); *input = __input; __read }
    /// // any other loop
    /// #read(input)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `read` - Emits the read, given the cursor it advances
    ///
    /// # Returns
    ///
    /// The read expression
    pub(super) fn read_input(self, read: impl FnOnce(&TokenStream) -> TokenStream) -> TokenStream {
        match self {
            Self::OneShot(CursorSite::Local) => {
                let read = read(&quote! { &mut __input });
                quote! {
                    {
                        let mut __input = *input;
                        let __read = #read;
                        *input = __input;
                        __read
                    }
                }
            }
            Self::Streaming | Self::OneShot(CursorSite::Caller) => read(&quote! { input }),
        }
    }

    /// Emits the handling of a failed fused-header parse, or of a value read from the main stream
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

    /// Emits the fatal exit for a malformed value, with no finalize: one read from the main stream
    /// ([`Self::on_parse_fail`]), or the bounded value of a `Sliced` unknown frame that the
    /// `#[klv(unknown)]` field keeps
    ///
    /// Both drivers name what `label` names first. Streaming rejects with `label`
    /// ([`Self::reject_label`]). One-shot relabels the value decoder's error with `label` through
    /// `tinyklv::__export::relabeled_error`, where the rule is stated
    ///
    /// # Arguments
    ///
    /// * `label` - Names what is rejected (see [`ParseSite::Value`])
    ///
    /// # Returns
    ///
    /// `return Err(concat!("invalid ", #label));` (streaming) or `__err` relabelled with `label`,
    /// after the [`CursorSite`]'s error exit (one-shot)
    pub(super) fn reject_value(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => self.reject_label(label),
            Self::OneShot(cursor) => {
                let exit = cursor.error_exit();
                quote! {
                    #exit ::core::result::Result::Err(::tinyklv::__export::relabeled_error(
                        input,
                        &checkpoint_inner,
                        #label,
                        __err,
                    ));
                }
            }
        }
    }

    /// Emits the fatal exit for a field whose `break_on` returned `Abort(__msg)`, with no
    /// finalize
    ///
    /// Both drivers name the field first, as for a malformed `Inline` value
    /// ([`Self::reject_value`]); one-shot keeps `__msg` as the label after it
    ///
    /// # Arguments
    ///
    /// * `label` - Names the field (a [`Self::reject_label`] label)
    ///
    /// # Returns
    ///
    /// `return Err(concat!("invalid ", #label));` (streaming) or a `ContextError` labelled
    /// `__msg`, relabelled with `label` (one-shot)
    pub(super) fn reject_aborted_value(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => self.reject_label(label),
            Self::OneShot(_) => {
                let reject = self.reject_value(label);
                quote! {
                    let __err = ::tinyklv::__export::labeled_error(input, &checkpoint_inner, __msg);
                    #reject
                }
            }
        }
    }
}

/// [`LoopCtx`] implementation
impl LoopCtx<'_> {
    /// Emits a pre-checked `take` of the frame's value length in bytes, or of an `implicit`
    /// value's `N`, off the main input
    ///
    /// The pre-check classes a short frame as truncation before `take` runs, so the `take` failure
    /// arm of a bound `subinput` is unreachable in practice; it finalizes (streaming) or rewinds
    /// and stops (one-shot). A short frame bound for `subinput` by its length first meets the
    /// `size(exact = N)` checks ([`Self::exact_guards`]), so a wrong length is rejected rather than
    /// classed as truncation. The rewind comes before them, so either exit leaves the cursor at
    /// the frame start
    ///
    /// ```text
    /// if input.eof_offset() < #len {
    ///     input.reset(&checkpoint_inner);
    ///     #exact_guards   // `TakeInto::Subinput` only
    ///     #clean_eof
    /// }
    /// #bind               // `subinput`, or dropped
    /// ```
    ///
    /// # Arguments
    ///
    /// * `into` - How many bytes are cut, and whether they are bound as `subinput` or dropped
    ///
    /// # Returns
    ///
    /// The pre-check followed by the `take` statement
    pub(super) fn take(self, into: TakeInto) -> TokenStream {
        let Self { driver, stream, .. } = self;
        let len = match into {
            TakeInto::Subinput | TakeInto::Discard => self.framing.len().into_token_stream(),
            TakeInto::Exact(width) => width.into_token_stream(),
        };
        // --------------------------------------------------
        // the `take` of the value bytes off the input
        // --------------------------------------------------
        let take = quote! {
            ::tinyklv::__export::winnow::token::take::<
                usize,
                #stream,
                ::tinyklv::__export::winnow::error::ContextError,
            >(#len).parse_next(input)
        };
        // --------------------------------------------------
        // bind the taken bytes as `subinput`, after the exact checks on a short frame, or drop
        // them
        // --------------------------------------------------
        let (guards, bind) = match into {
            TakeInto::Subinput => (Some(self.exact_guards()), self.bind_subinput(&take)),
            TakeInto::Exact(_) => (None, self.bind_subinput(&take)),
            TakeInto::Discard => (None, quote! { let _ = #take; }),
        };
        // --------------------------------------------------
        // pre-check the length, so a short frame is rewound and then rejected or truncation,
        // then bind
        // --------------------------------------------------
        let rewind = LoopDriver::rewind();
        let exit = driver.clean_eof();
        quote! {
            if input.eof_offset() < #len {
                #rewind
                #guards
                #exit
            }
            #bind
        }
    }

    /// Emits the binding of a pre-checked `take`'s bytes as `subinput`
    ///
    /// A failed `take` finalizes (streaming) or rewinds and stops (one-shot)
    ///
    /// # Arguments
    ///
    /// * `take` - The `take` expression
    ///
    /// # Returns
    ///
    /// The `let mut subinput` statement
    fn bind_subinput(self, take: &TokenStream) -> TokenStream {
        let Self { driver, stream, .. } = self;
        let fail_arm = match driver {
            LoopDriver::Streaming => {
                let finish = self.finish();
                quote! { return #finish, }
            }
            LoopDriver::OneShot(_) => {
                let truncated = driver.truncated();
                quote! { { #truncated } }
            }
        };
        quote! {
            let mut subinput: <#stream as ::tinyklv::__export::winnow::stream::Stream>::Slice = match #take {
                ::core::result::Result::Ok(s) => s,
                ::core::result::Result::Err(_) => #fail_arm
            };
        }
    }

    /// Emits the `break_on` check that runs right after the frame header, when `break_on` is set
    ///
    /// * separate frame whose header reads the length ([`LenRead::Hoisted`]) - a `match` on the
    ///   `BreakType` that the key literal (`Done` when the key equals it, else `Proceed`) or the
    ///   user's `fn(key, len)` returns ([`Self::break_match`]); `Done` ends the loop with the
    ///   key and length consumed, before any value byte is read
    /// * separate frame of a container with a `size(implicit)` field or `unknown(..)` - only a key
    ///   literal, as `if #key == #lit { break; }`, checked right after the key: the loop ends
    ///   before the frame's length, if it has one, is read. The derive does not read every such
    ///   frame's length, so a `break_on` fn is rejected at parse time
    ///   ([`MainContainer::reads_every_len`])
    /// * fused frame - only a key literal, as above, checked once the header is consumed, before
    ///   any value byte is read
    ///
    /// Every form can `break`, so a `Some` is the report
    /// [`DecodeCtx::breaks`](super::DecodeCtx::breaks) ORs
    ///
    /// # Returns
    ///
    /// The check, or `None` when `break_on` is unset
    pub(super) fn break_on(self) -> Option<TokenStream> {
        let key = self.framing.key();
        let stop_at = |lit: &syn::Lit| {
            quote! {
                if #key == #lit {
                    break;
                }
            }
        };
        match self.framing {
            DecodeFraming::Separate {
                break_on, len_read, ..
            } => match (break_on?, len_read) {
                (BreakOnSpec::Literal(lit), LenRead::Hoisted) => Some(self.break_match(&quote! {
                    if #key == #lit {
                        ::tinyklv::BreakType::Done
                    } else {
                        ::tinyklv::BreakType::Proceed
                    }
                })),
                (BreakOnSpec::Func(path), LenRead::Hoisted) => {
                    let len = self.framing.len();
                    Some(self.break_match(&quote! { #path(#key, #len) }))
                }
                (BreakOnSpec::Literal(lit), LenRead::UnlessImplicit | LenRead::IfPrefixed(_)) => {
                    Some(stop_at(lit))
                }
                // --------------------------------------------------
                // rejected at parse time: the derive does not read every frame's length
                // --------------------------------------------------
                (BreakOnSpec::Func(_), LenRead::UnlessImplicit | LenRead::IfPrefixed(_)) => None,
            },
            DecodeFraming::Fused { break_on, .. } => break_on.map(stop_at),
        }
    }

    /// Emits the `match` on a separate frame's `break_on` decision
    ///
    /// `Skip` discards the value via [`Self::take`] and continues, `Done` ends the loop right
    /// after the header (no rewind, the header stays consumed), `Abort` is a
    /// [`LoopDriver::reject`] with the user's message, and `Proceed` (and any future variant) falls
    /// through to the value dispatch
    ///
    /// # Arguments
    ///
    /// * `decision` - The per-iteration `BreakType` scrutinee
    ///
    /// # Returns
    ///
    /// The break-handling `match`
    fn break_match(self, decision: &TokenStream) -> TokenStream {
        // --------------------------------------------------
        // `Skip` discards the value bytes
        // --------------------------------------------------
        let skip = self.take(TakeInto::Discard);
        // --------------------------------------------------
        // `Abort` rejects with the user's message
        // --------------------------------------------------
        let abort = self.driver.reject(&quote! { __msg });
        // --------------------------------------------------
        // match the decision; `Proceed` falls through to the value dispatch
        // --------------------------------------------------
        quote! {
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
        }
    }

    /// Emits the tail expression that finalizes `__acc` once the loop ends
    ///
    /// # Returns
    ///
    /// `Ok(Ready(v))` / `Err(label)` (streaming) or the struct with a missing-field label mapped
    /// to a `ContextError` (one-shot). A one-shot loop over a [`CursorSite::Local`] cursor first
    /// takes the label held in `__exact_reject`, if any, through the same mapping
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
            LoopDriver::OneShot(CursorSite::Caller) => quote! {
                <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc)
                    .map_err(|label| ::tinyklv::__export::labeled_error(input, &checkpoint, label))
            },
            LoopDriver::OneShot(CursorSite::Local) => quote! {
                match __exact_reject {
                    ::core::option::Option::Some(label) => ::core::result::Result::Err(label),
                    ::core::option::Option::None => <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc),
                }
                .map_err(|label| ::tinyklv::__export::labeled_error(input, &checkpoint, label))
            },
        }
    }

    /// Generates the label naming what the derive rejects, and why
    ///
    /// A noun phrase, so that it reads after the `invalid ` that the streaming driver prefixes and
    /// that winnow's `ContextError` display prints before a one-shot label
    ///
    /// # Arguments
    ///
    /// * `field` - The field whose value is rejected, `None` for the frame as a whole
    /// * `reason` - Why, written in the parentheses after what is rejected
    ///
    /// # Returns
    ///
    /// A [`LoopDriver::reject_label`] label, `` `<container>::<field>` value (<reason>) `` or
    /// `` `<container>` frame (<reason>) ``
    pub(super) fn label(self, field: Option<&syn::Ident>, reason: &str) -> TokenStream {
        let container = &self.input.ident;
        let (field, rejected) = match field {
            Some(field) => (Some(quote! { "::", ::core::stringify!(#field), }), "value"),
            None => (None, "frame"),
        };
        let tail = format!("` {rejected} ({reason})");
        quote! {
            ::core::concat!(
                "`",
                ::core::stringify!(#container),
                #field
                #tail,
            )
        }
    }
}
