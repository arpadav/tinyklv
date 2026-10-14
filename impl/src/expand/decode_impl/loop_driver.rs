//! The exit policy of a generated decode loop
//!
//! Both decode entry points run the same per-frame loop and differ only in how an iteration leaves
//! it. [`LoopDriver`] owns the exits every framing shares, so each is spelled once per driver. The
//! key + len streaming header is the exception: it finalizes on a malformed key (see
//! `LoopCtx::header`):
//!
//! * [`LoopDriver::Streaming`] - `resume_partial`: truncation rewinds and returns
//!   `Packet::NeedMore`, an error returns a `tinyklv::traits::PartialError` of a label and its
//!   kind, the label a one-shot error carries for the same rejection
//!   ([`LoopDriver::reject_label`])
//! * [`LoopDriver::OneShot`] - `decode_value`: truncation rewinds and `break`s to the finalize
//!   tail, an error leaves the fn with a `ContextError` labelled at the input position, except a
//!   malformed value read from the main stream, kept from a `Sliced` unknown frame or, under
//!   `strict`, cut by a frame's length, whose error names the field (or an unknown frame) first,
//!   then the value decoder's own error ([`LoopDriver::reject_value`]). Its [`CursorSite`] says
//!   which cursor the loop advances, and so how an error leaves the fn
//!
//! The rules that decide truncation against malformed are stated once, on `tinyklv::Framing`
//! and `tinyklv::Framing::Inline`
//!
//! The exits that slice or finalize also name the stream and accumulator types, so they live on
//! [`LoopCtx`], which pairs a driver with those types
//!
//! The emitted tokens assume the loop's bindings: `input`, `__checkpoint` (fn entry),
//! `__checkpoint_inner` (frame start), `__acc` (the partial accumulator), the frame's value length
//! (`DecodeFraming::len`, wherever a `take` is cut or a `size(exact = N)` check reads it),
//! inside a failed main-stream, kept `Sliced` unknown or `strict` sliced value arm, `__err` (the
//! value decoder's error), inside the `Abort` arm of a field's `break_on`, `__msg` (its message)
//! and, in a one-shot loop over a [`CursorSite::Local`] cursor, `__exact_reject` (the held
//! `size(exact = N)` rejection)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{ErrorKind, LoopCtx, framing::DecodeFraming};
use crate::{
    ast::attr::{FieldKey, MainContainer, container::BreakOnSpec},
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};

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
    /// [`Self::Local`] when a key's frame length is checked against its `size(exact = N)`
    /// ([`Self::exact_keys`]) and the stream is `&[u8]`, else [`Self::Caller`]: a container
    /// without such a key decodes over the caller's cursor in fewer instructions, and any other
    /// stream need not be `Clone`
    pub(super) fn of(input: &MainContainer, stream: &syn::Type) -> Self {
        if Self::exact_keys(input).next().is_some() && super::is_u8_slice(stream) {
            Self::Local
        } else {
            Self::Caller
        }
    }

    /// The keys whose frame length is checked against their `size(exact = N)`
    /// ([`SizeSpec::len_check`](crate::ast::attr::size::SizeSpec::len_check)), which pick the
    /// cursor and which a frame running past the end of the input meets
    /// ([`LoopCtx::exact_guards`])
    ///
    /// # Arguments
    ///
    /// * `input` - The container
    ///
    /// # Returns
    ///
    /// Each such key in field order, with its `N` and `N` as the field wrote it
    pub(super) fn exact_keys<'a>(
        input: &'a MainContainer<'a>,
    ) -> impl Iterator<Item = (FieldKey<'a>, usize, &'a syn::LitInt)> {
        input.klv_keys().filter_map(|key| {
            let (width, lit) = key.codec.size.as_ref()?.len_check()?;
            Some((key, width, lit))
        })
    }

    /// The label of the block a [`Self::Local`] loop runs in, which every error exit leaves
    ///
    /// # Returns
    ///
    /// `'__tinyklv_decode`
    fn label() -> syn::Lifetime {
        syn::Lifetime::new("'__tinyklv_decode", proc_macro2::Span::call_site())
    }

    /// Emits the exit an error leaves the decode fn with, carrying `value`
    ///
    /// # Arguments
    ///
    /// * `value` - The `Err` the decode fn returns
    ///
    /// # Returns
    ///
    /// `return #value;` (caller) or `break '__tinyklv_decode (#value);` (local), whose value is
    /// parenthesised: syn reads a path's leading `::` right after a label as the `:` of a labelled
    /// loop
    fn error_exit(self, value: &TokenStream) -> TokenStream {
        match self {
            Self::Caller => quote! { return #value; },
            Self::Local => {
                let label = Self::label();
                quote! { break #label (#value); }
            }
        }
    }

    /// Emits the body of the one-shot `decode_value` fn: its frame loop over the caller's cursor,
    /// or over a local copy inside the block [`Self::label`] labels, which every exit leaves and
    /// after which the copy is written back to the caller's
    ///
    /// # Arguments
    ///
    /// * `partial_ty` - The partial accumulator type, whose default `__acc` starts at
    /// * `frame_loop` - The loop and its tail ([`LoopCtx::frame_loop`])
    ///
    /// # Returns
    ///
    /// The fn body
    pub(super) fn decode_value_body(
        self,
        partial_ty: &TokenStream,
        frame_loop: &TokenStream,
    ) -> TokenStream {
        match self {
            Self::Caller => quote! {
                let __checkpoint = ::tinyklv::__export::winnow::stream::Stream::checkpoint(input);
                let mut __acc = <#partial_ty as ::core::default::Default>::default();
                #frame_loop
            },
            Self::Local => {
                let label = Self::label();
                quote! {
                    let mut __cursor = ::core::clone::Clone::clone(&*input);
                    let __result = #label: {
                        let input = &mut __cursor;
                        let __checkpoint = ::tinyklv::__export::winnow::stream::Stream::checkpoint(input);
                        let mut __acc = <#partial_ty as ::core::default::Default>::default();
                        let mut __exact_reject: ::core::option::Option<::tinyklv::traits::PartialError> = ::core::option::Option::None;
                        #frame_loop
                    };
                    *input = __cursor;
                    __result
                }
            }
        }
    }
}

/// How many bytes [`LoopCtx::take`] cuts off the input, and what it does with them
#[derive(Debug, Clone, Copy)]
pub(super) enum TakeInto {
    /// Bind the frame's length in bytes as `__subinput`, the bounded slice a known-key arm, or the
    /// `unknown` decoder of a kept `Sliced` frame, decodes from; a frame running past the end of
    /// the input first meets the `size(exact = N)` checks ([`LoopCtx::exact_guards`])
    Subinput,
    /// Bind `N` bytes as `__subinput` for a `size(implicit, exact = N)` value, whose frame has no
    /// length to check
    Exact(usize),
    /// Drop the frame's length in bytes, which no field reads
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
        /// item, which a malformed value is rejected with (a [`LoopDriver::reject_label`] label)
        label: &'a TokenStream,
    },
}

/// [`LoopDriver`] implementation
impl LoopDriver {
    /// Emits the exit taken when the input is empty at the top of an iteration
    ///
    /// Nothing was consumed, so there is nothing to rewind. The key + len streaming header
    /// also takes it, after its rewind, when the key decoder fails at the end of the input or
    /// without consuming a byte
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
    /// A streaming loop reaches it only through a fused frame's unknown `Inline` key; a one-shot
    /// loop already breaks at [`Self::clean_eof`]
    ///
    /// # Returns
    ///
    /// `Stream::reset(input, &__checkpoint_inner); break;`
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
    /// `Stream::reset(input, &__checkpoint_inner);`
    fn rewind() -> TokenStream {
        quote! { ::tinyklv::__export::winnow::stream::Stream::reset(input, &__checkpoint_inner); }
    }

    /// Emits a fatal exit, with no finalize: streaming returns `partial_error`, one-shot an error
    /// labelled `label` and tagged with `kind`
    ///
    /// A `break_on`'s `Abort` leaves through it too. `Abort` is documented to *guarantee* an `Err`
    /// with no partially-decoded value surfaced, so this exit honors that contract rather than
    /// finalizing what was accumulated; both drivers leave with `Err` without finalizing
    ///
    /// # Arguments
    ///
    /// * `label` - An expression evaluating to the `&'static str` label of the one-shot error
    /// * `kind` - What the one-shot error is
    /// * `partial_error` - The `tinyklv::traits::PartialError` streaming returns, of `kind`
    ///
    /// # Returns
    ///
    /// `return Err(#partial_error);` (streaming) or `Err(kinded_error(input, &__checkpoint, #label,
    /// #kind))` through the [`CursorSite`]'s error exit (one-shot)
    fn reject(
        self,
        label: &TokenStream,
        kind: ErrorKind,
        partial_error: &TokenStream,
    ) -> TokenStream {
        match self {
            Self::Streaming => quote! { return ::core::result::Result::Err(#partial_error); },
            Self::OneShot(cursor) => cursor.error_exit(&quote! {
                ::core::result::Result::Err(::tinyklv::__export::kinded_error(
                    input,
                    &__checkpoint,
                    #label,
                    #kind,
                ))
            }),
        }
    }

    /// Emits a fatal exit naming `label`, with no finalize ([`Self::reject`])
    ///
    /// Both drivers label the error `label`, which a `ContextError`'s display prints after
    /// `invalid `: the one-shot error's, and the one a `tinyklv::Decoder` turns the streaming
    /// error into
    ///
    /// # Arguments
    ///
    /// * `label` - A `::core::concat!(..)` of string literals naming what was rejected, so that
    ///   the streaming error is a constant
    /// * `kind` - What the error is
    ///
    /// # Returns
    ///
    /// The [`Self::reject`] exit, streaming's error `PartialError::new(&(#kind, #label))`, a
    /// constant
    pub(super) fn reject_label(self, label: &TokenStream, kind: ErrorKind) -> TokenStream {
        let partial_error = quote! {
            ::tinyklv::traits::PartialError::new(&(#kind, #label))
        };
        self.reject(label, kind, &partial_error)
    }

    /// Emits the fatal exit of a `break_on` that returned `Abort(__msg)`, with no finalize
    /// ([`Self::reject`]), labelled with `__msg` and of [`ErrorKind::Aborted`]
    ///
    /// # Returns
    ///
    /// The [`Self::reject`] exit, streaming's error `PartialError::aborted(__msg)`
    pub(super) fn abort(self) -> TokenStream {
        self.reject(
            &quote! { __msg },
            ErrorKind::Aborted,
            &quote! { ::tinyklv::traits::PartialError::aborted(__msg) },
        )
    }

    /// Emits the fatal exit for a frame whose length is not its field's `size(exact = N)`, with
    /// no finalize
    ///
    /// A one-shot loop over a [`CursorSite::Local`] cursor holds the error in `__exact_reject` for
    /// [`LoopCtx::finish`] to return, so every such rejection shares its one error exit, and
    /// leaves the loop; any other loop leaves through [`Self::reject_label`]. Either error is
    /// [`ErrorKind::WrongLength`]
    ///
    /// # Arguments
    ///
    /// * `label` - Names the field and its `N` (a [`Self::reject_label`] label)
    ///
    /// # Returns
    ///
    /// `__exact_reject = Some(PartialError::new(&(WrongLength, #label))); break;` (one-shot, local
    /// cursor) or the [`Self::reject_label`] exit
    pub(super) fn reject_exact(self, label: &TokenStream) -> TokenStream {
        let kind = ErrorKind::WrongLength;
        match self {
            Self::OneShot(CursorSite::Local) => quote! {
                __exact_reject = ::core::option::Option::Some(
                    ::tinyklv::traits::PartialError::new(&(#kind, #label))
                );
                break;
            },
            Self::Streaming | Self::OneShot(CursorSite::Caller) => self.reject_label(label, kind),
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
                let malformed = self.reject_value(label, ErrorKind::MalformedValue);
                quote! {
                    if ::tinyklv::__export::winnow::stream::Stream::eof_offset(input) == 0 {
                        #truncated
                    }
                    #malformed
                }
            }
        }
    }

    /// Emits the fatal exit for a malformed value, with no finalize: one read from the main stream
    /// ([`Self::on_parse_fail`]), the bounded value of a `Sliced` unknown frame that the
    /// `#[klv(unknown)]` field keeps, or, under `strict`, a field's value cut by its frame's length
    ///
    /// Both drivers name what `label` names first. Streaming rejects with `label`
    /// ([`Self::reject_label`]). One-shot relabels the value decoder's error with `label` through
    /// `tinyklv::__export::relabeled_error`, where the rule is stated
    ///
    /// # Arguments
    ///
    /// * `label` - Names what is rejected (see [`ParseSite::Value`])
    /// * `kind` - What the error is
    ///
    /// # Returns
    ///
    /// The [`Self::reject_label`] exit (streaming) or `__err` relabelled with `label` and tagged
    /// with `kind`, through the [`CursorSite`]'s error exit (one-shot)
    pub(super) fn reject_value(self, label: &TokenStream, kind: ErrorKind) -> TokenStream {
        match self {
            Self::Streaming => self.reject_label(label, kind),
            Self::OneShot(cursor) => cursor.error_exit(&quote! {
                ::core::result::Result::Err(::tinyklv::__export::relabeled_error(
                    input,
                    &__checkpoint_inner,
                    #label,
                    __err,
                    #kind,
                ))
            }),
        }
    }

    /// Emits the fatal exit for a field whose `break_on` returned `Abort(__msg)`, with no
    /// finalize
    ///
    /// Both drivers keep `__msg`, as for the container's `break_on` ([`Self::abort`]). One-shot
    /// names the field first, as for a malformed `Inline` value ([`Self::reject_value`]). Either
    /// error is [`ErrorKind::Aborted`]
    ///
    /// # Arguments
    ///
    /// * `label` - Names the field (a [`Self::reject_label`] label), for one-shot
    ///
    /// # Returns
    ///
    /// The [`Self::abort`] exit (streaming) or a `ContextError` labelled `__msg`, relabelled with
    /// `label` (one-shot)
    pub(super) fn reject_aborted_value(self, label: &TokenStream) -> TokenStream {
        match self {
            Self::Streaming => self.abort(),
            Self::OneShot(_) => {
                let reject = self.reject_value(label, ErrorKind::Aborted);
                quote! {
                    let __err = ::tinyklv::__export::labeled_error(input, &__checkpoint_inner, __msg);
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
    /// arm of a bound `__subinput` is unreachable in practice; it finalizes (streaming) or rewinds
    /// and stops (one-shot). A short frame bound for `__subinput` by its length first meets the
    /// `size(exact = N)` checks ([`Self::exact_guards`]), so a wrong length is rejected rather than
    /// classed as truncation. The rewind comes before them, so either exit leaves the cursor at
    /// the frame start
    ///
    /// ```text
    /// if Stream::eof_offset(input) < #len {
    ///     Stream::reset(input, &__checkpoint_inner);
    ///     #exact_guards   // `TakeInto::Subinput` only
    ///     #clean_eof
    /// }
    /// #bind               // `__subinput`, or dropped
    /// ```
    ///
    /// # Arguments
    ///
    /// * `into` - How many bytes are cut, and whether they are bound as `__subinput` or dropped
    ///
    /// # Returns
    ///
    /// The pre-check followed by the `take` statement
    pub(super) fn take(self, into: TakeInto) -> TokenStream {
        let Self { driver, stream, .. } = self;
        let len = match into {
            TakeInto::Subinput | TakeInto::Discard => DecodeFraming::len().into_token_stream(),
            TakeInto::Exact(width) => width.into_token_stream(),
        };
        // --------------------------------------------------
        // the `take` of the value bytes off the input
        // --------------------------------------------------
        let take = quote! {
            ::tinyklv::__export::winnow::Parser::parse_next(
                &mut ::tinyklv::__export::winnow::token::take::<
                    ::core::primitive::usize,
                    #stream,
                    ::tinyklv::__export::winnow::error::ContextError,
                >(#len),
                input,
            )
        };
        // --------------------------------------------------
        // bind the taken bytes as `__subinput`, after the exact checks on a short frame, or drop
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
            if ::tinyklv::__export::winnow::stream::Stream::eof_offset(input) < #len {
                #rewind
                #guards
                #exit
            }
            #bind
        }
    }

    /// Emits the binding of a pre-checked `take`'s bytes as `__subinput`
    ///
    /// A failed `take` finalizes (streaming) or rewinds and stops (one-shot)
    ///
    /// # Arguments
    ///
    /// * `take` - The `take` expression
    ///
    /// # Returns
    ///
    /// The `let mut __subinput` statement
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
            let mut __subinput: <#stream as ::tinyklv::__export::winnow::stream::Stream>::Slice = match #take {
                ::core::result::Result::Ok(__subinput) => __subinput,
                ::core::result::Result::Err(_) => #fail_arm
            };
        }
    }

    /// Emits the `break_on` check that runs right after the frame's key, when `break_on` is set,
    /// with the rest of the frame: after a separate key, before its length, or after a fused
    /// header, which reads the length with the key
    ///
    /// ```text
    /// // a key literal
    /// if #key == #lit {
    ///     break;
    /// }
    /// #frame
    /// // the user's `fn(&key, &mut input) -> BreakType`
    /// let __after_key = Stream::checkpoint(input);
    /// match { let __decision: BreakType = #path(&#key, input); __decision } {
    ///     Skip => {
    ///         #progress_check
    ///         continue;
    ///     }
    ///     Done => break,
    ///     Abort(__msg) => {
    ///         #reject
    ///     }
    ///     _ => {
    ///         Stream::reset(input, &__after_key);
    ///         #frame
    ///     }
    /// }
    /// // a fused header's `Skip` and `Done` first step over an item the fn read nothing of
    /// Skip | Done => {
    ///     if Offset::offset_from(&*input, &__after_key) == 0 {
    ///         let Sliced(__len) = __framing else { break; };
    ///         #take(Discard)
    ///     }
    ///     break; // `Done` only
    /// }
    /// ```
    ///
    /// The key literal ends the loop with the input right after the key. The fn may read past the
    /// key: `Proceed`, like any later `BreakType` variant, rewinds to just after the key, where
    /// the frame then decodes; `Skip` continues, rejecting first a frame that moved the cursor
    /// nowhere, which the loop would repeat forever ([`Self::progress_check`]), `Done` ends the
    /// loop and `Abort` rejects with the fn's message ([`LoopDriver::abort`]), each with the input
    /// where the fn left it. A fused header has read the item's length, which the fn is not
    /// handed, so a `Skip` or `Done` that left the input right after the header steps over a
    /// `Sliced` item's value, and ends the loop there after an `Inline` one, which has no length
    /// to step over it by. A fused `Skip` then leaves the `match` for the progress check that
    /// follows every fused frame ([`Self::frame_loop`]), since a check of its own in the arm
    /// costs instructions on every frame that proceeds. The fn is handed the cursor
    /// [`LoopDriver::read_input`] gives, and its call, that cursor and the `BreakType` it must
    /// return are located at the fn, so a fn of another signature is reported there
    ///
    /// # Arguments
    ///
    /// * `frame` - The rest of the frame, decoded once the check lets it proceed
    ///   ([`Self::after_key`])
    ///
    /// # Returns
    ///
    /// The check and `frame`, or `frame` itself when `break_on` is unset
    pub(super) fn break_on(self, frame: TokenStream) -> TokenStream {
        let mut key = DecodeFraming::key();
        // --------------------------------------------------
        // a key literal ends the loop on its key
        // --------------------------------------------------
        let path = match self.input.attrs.break_on.as_ref() {
            None => return frame,
            Some(BreakOnSpec::Literal(lit)) => {
                return quote! {
                    if #key == #lit {
                        break;
                    }
                    #frame
                };
            }
            Some(BreakOnSpec::Func(path)) => path,
        };
        // --------------------------------------------------
        // the fn's call, located at the fn, and its `Abort`, which rejects with its message
        // --------------------------------------------------
        let at_fn = helpers::call_site_at(path);
        key.set_span(at_fn);
        let decision = self.driver.read_input(|cursor| {
            let cursor = cursor.clone().into_iter().map(|mut token| {
                token.set_span(at_fn);
                token
            });
            quote_spanned! {at_fn=>
                { let __decision: ::tinyklv::BreakType = #path(&#key, #(#cursor)*); __decision }
            }
        });
        let abort = self.driver.abort();
        // --------------------------------------------------
        // a fused header has read the item's length, which the fn is not handed: a `Skip` or
        // `Done` that read nothing steps over a `Sliced` item, and an `Inline` one, with no
        // length to step over it by, ends the loop after its header. A `Skip` repeats the loop,
        // so an item that moved the cursor nowhere is rejected first: a separate `Skip` by its
        // own check, a fused one by the check after the frame
        // --------------------------------------------------
        let (skip, done) = match self.framing {
            DecodeFraming::Fused { .. } => {
                let len = DecodeFraming::len();
                let take = self.take(TakeInto::Discard);
                let step_over = quote! {
                    if ::tinyklv::__export::winnow::stream::Offset::offset_from(&*input, &__after_key) == 0 {
                        let ::tinyklv::Framing::Sliced(#len) = __framing else {
                            break;
                        };
                        #take
                    }
                };
                (quote! {{ #step_over }}, quote! {{ #step_over break; }})
            }
            DecodeFraming::Separate { .. } => {
                let progress = self.progress_check();
                (quote! {{ #progress continue; }}, quote! { break })
            }
        };
        // --------------------------------------------------
        // match the decision, from a checkpoint just after the key
        // --------------------------------------------------
        quote! {
            let __after_key = ::tinyklv::__export::winnow::stream::Stream::checkpoint(input);
            match #decision {
                ::tinyklv::BreakType::Skip => #skip,
                ::tinyklv::BreakType::Done => #done,
                ::tinyklv::BreakType::Abort(__msg) => {
                    #abort
                }
                // `Proceed`, and any later `BreakType` variant, decodes the frame from just after
                // its key
                _ => {
                    ::tinyklv::__export::winnow::stream::Stream::reset(input, &__after_key);
                    #frame
                }
            }
        }
    }

    /// Emits the tail expression that finalizes `__acc` once the loop ends
    ///
    /// # Returns
    ///
    /// `Ok(Ready(v))` / `Err(__err)` (streaming) or the struct with a missing-field error mapped
    /// to a `ContextError` labelled with its label and tagged with its kind (one-shot). A one-shot
    /// loop over a [`CursorSite::Local`] cursor first takes the error held in `__exact_reject`, if
    /// any, through the same mapping. Over the caller's cursor the error can only be the
    /// generated `finalize`'s, so its tag names [`ErrorKind::MissingRequired`] as a constant
    /// rather than reading the kind back
    pub(super) fn finish(self) -> TokenStream {
        let partial_ty = self.partial_ty;
        let to_context = |kind: &TokenStream| {
            quote! {
                |__err| ::tinyklv::__export::kinded_error(input, &__checkpoint, __err.label(), #kind)
            }
        };
        match self.driver {
            LoopDriver::Streaming => quote! {
                match <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc) {
                    ::core::result::Result::Ok(__val) => ::core::result::Result::Ok(
                        ::tinyklv::decoder::Packet::Ready(__val)
                    ),
                    ::core::result::Result::Err(__err) => ::core::result::Result::Err(__err),
                }
            },
            LoopDriver::OneShot(CursorSite::Caller) => {
                let to_context = to_context(&ErrorKind::MissingRequired.into_token_stream());
                quote! {
                    <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc).map_err(#to_context)
                }
            }
            LoopDriver::OneShot(CursorSite::Local) => {
                let to_context = to_context(&quote! { __err.kind() });
                quote! {
                    match __exact_reject {
                        ::core::option::Option::Some(__err) => ::core::result::Result::Err(__err),
                        ::core::option::Option::None => <#partial_ty as ::tinyklv::traits::Partial>::finalize(__acc),
                    }
                    .map_err(#to_context)
                }
            }
        }
    }

    /// Generates the label naming what the derive rejects, and why
    ///
    /// A noun phrase, so that it reads after the `invalid ` that winnow's `ContextError` display
    /// prints before it
    ///
    /// # Arguments
    ///
    /// * `field` - The field whose value is rejected, `None` for the item as a whole
    /// * `reason` - Why, written in the parentheses after what is rejected
    ///
    /// # Returns
    ///
    /// A [`LoopDriver::reject_label`] label, `` `<container>::<field>` value (<reason>) `` or
    /// `` `<container>` item (<reason>) ``
    pub(super) fn label(self, field: Option<&syn::Ident>, reason: &str) -> TokenStream {
        let container = &self.input.ident;
        let (field, rejected) = match field {
            Some(field) => (Some(quote! { "::", ::core::stringify!(#field), }), "value"),
            None => (None, "item"),
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
