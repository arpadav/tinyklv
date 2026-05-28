//! Code generation for `DecodePartial` and `ResumePartial` trait implementations
//!
//! Provides two entry points consumed by [`super::gen_decode_impl`]:
//!
//! * [`gen_decode_partial_impl`] - emits the `DecodePartial` impl, which
//!   allocates a default partial struct and delegates to `ResumePartial`
//! * [`gen_resume_partial_impl`] - emits the `ResumePartial` impl, which
//!   contains the main streaming decode loop: key/len parsing, break-condition
//!   dispatch, value slicing, and per-field partial accumulation
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{constants, key_match_gen};
use crate::ast::{attr::MainContainer, types};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates the `DecodePartial` trait implementation for a container
///
/// The emitted impl delegates entirely to `ResumePartial::resume_partial`,
/// constructing a default partial struct and handing control over. This keeps
/// the `DecodePartial` impl thin and ensures fresh-start and mid-stream
/// resumption share identical logic
///
/// # Arguments
///
/// * `name` - The container ident the impl is generated for
/// * `partial_name` - The associated partial-packet struct ident
/// * `stream` - The stream type (e.g. `&[u8]`) the impl is parameterised over
/// * `generics` - Generic parameters from the original struct definition
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the complete `DecodePartial` impl block
pub(super) fn gen_decode_partial_impl(
    name: &syn::Ident,
    partial_name: &syn::Ident,
    stream: &syn::Type,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // split generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // fresh-start dispatch: build a default partial struct
    // and hand off to resume entry
    // --------------------------------------------------
    quote! {
        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::DecodePartial`] for [`", stringify!(#stream), "`]")]
        impl #impl_generics ::tinyklv::traits::DecodePartial<#stream> for #name #ty_generics #where_clause {
            type Partial = #partial_name #ty_generics;

            #[inline(always)]
            fn decode_partial(
                input: &mut #stream,
            ) -> ::core::result::Result<
                ::tinyklv::decoder::Packet<Self, Self::Partial>,
                &'static str,
            > {
                <Self as ::tinyklv::traits::ResumePartial<#stream>>::resume_partial(
                    input,
                    <#partial_name #ty_generics as ::core::default::Default>::default(),
                )
            }
        }
    }
}

/// Generates the `ResumePartial` trait implementation containing the main decode loop
///
/// The emitted impl is the core of the streaming decoder. On each call it:
///
/// 1. Checks for EOF and surfaces a `NeedMore` if no bytes are available
/// 2. Parses the key, distinguishing zero-consumed truncation from key malformation
/// 3. Parses the length; on failure rewinds and surfaces `NeedMore`
/// 4. Dispatches to `break_condition`, handling `Proceed`, `Skip`, `Done`, and `Abort`
/// 5. Optionally checks the parsed key against the known-key set when `deny_unknown_keys` is set
/// 6. Slices the value bytes and dispatches to per-field decoders via the generated `match` arms
/// 7. When the loop exits (`Done`), calls `Partial::finalize` and returns `Ready` or an error label
///
/// # Arguments
///
/// * `input` - The fully parsed container, providing field data and container attributes
/// * `name` - The container ident the impl is generated for
/// * `partial_name` - The associated partial-packet struct ident
/// * `stream` - The stream type parameterising the impl
/// * `key_decoder` - Token expression used to decode each TLV key from the stream
/// * `len_decoder` - Token expression used to decode each TLV length from the stream
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the complete `ResumePartial` impl block
pub(super) fn gen_resume_partial_impl(
    input: &MainContainer,
    name: &syn::Ident,
    partial_name: &syn::Ident,
    stream: &syn::Type,
    key_decoder: &types::XcoderType,
    len_decoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // split generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    // --------------------------------------------------
    // container attributes
    // --------------------------------------------------
    let debug_on = input.attrs.debug.is_some();
    let deny_unknown_keys = input.attrs.deny_unknown_keys.is_some();
    // --------------------------------------------------
    // * field assignment during decode
    // * known-keys precheck - only if deny_unknown_keys is enabled
    // --------------------------------------------------
    let items_match = key_match_gen::gen_items_match(&input.data, stream, debug_on);
    let (pre_check_gate, pre_check_clippy_allow) = if deny_unknown_keys {
        (
            key_match_gen::gen_known_keys_check(name, &input.data),
            quote! {
                #[allow(
                    clippy::manual_range_patterns,
                    reason = "the matches!(key, K1 | K2 | ...) pattern is generated by the macro and cannot be avoided. this lint appears when keys are consecutive",
                )]
            },
        )
    } else {
        (quote! {}, quote! {})
    };
    // --------------------------------------------------
    // debug statement after key/len parse
    // --------------------------------------------------
    let debug_key_val = if debug_on {
        let logger = constants::logger();
        quote! {
            #logger ("key: {}, len: {}", key, len);
        }
    } else { quote! {} };
    // --------------------------------------------------
    // return impl - this is main decoding logic
    // --------------------------------------------------
    quote! {
        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::traits::ResumePartial`] for [`", stringify!(#stream), "`].")]
        impl #impl_generics ::tinyklv::traits::ResumePartial<#stream> for #name #ty_generics #where_clause {

            #pre_check_clippy_allow

            fn resume_partial(
                input: &mut #stream,
                mut __acc: #partial_name #ty_generics,
            ) -> ::core::result::Result<
                ::tinyklv::decoder::Packet<Self, #partial_name #ty_generics>,
                &'static str,
            > {
                let checkpoint = input.checkpoint();
                let _ = checkpoint;
                loop {
                    // --------------------------------------------------
                    // EOF: "the buffer just ran out", NOT "the packet
                    // is done". surface the in-flight partial via
                    // NeedMore so the caller can `feed` more bytes
                    // and resume. an entity does not know the stream
                    // is complete unless the user implements a `Done`
                    // break condition; without one, the caller owns
                    // the finalise call (typically via
                    // [`tinyklv::Decoder::finish`]).
                    // --------------------------------------------------
                    if input.eof_offset() == 0 {
                        return ::core::result::Result::Ok(
                            ::tinyklv::decoder::Packet::NeedMore(__acc)
                        );
                    }
                    let checkpoint_inner = input.checkpoint();
                    // --------------------------------------------------
                    // parse key then len sequentially. splitting the
                    // prefix lets us distinguish truncation from
                    // malformation:
                    //
                    // * key parse fails with zero bytes consumed ->
                    //   we ran out of input before the prefix could
                    //   start. surface as NeedMore so the caller can
                    //   `feed` more.
                    // * key parses but len parse fails -> we have a
                    //   partial prefix; len bytes are short, so this
                    //   is also truncation (NeedMore).
                    // * key parse fails with bytes consumed -> the
                    //   key decoder rejected the bytes it saw; this
                    //   is actual malformation (Malformed).
                    //
                    // a single `(key, len).parse_next` cannot tell
                    // these apart because the tuple's error path
                    // doesn't expose "which sub-parser" failed or how
                    // many bytes it consumed.
                    // --------------------------------------------------
                    let __key_offset_before = input.eof_offset();
                    let key = match #key_decoder.parse_next(input) {
                        Ok(k) => k,
                        Err(_) => {
                            let __consumed = __key_offset_before - input.eof_offset();
                            input.reset(&checkpoint_inner);
                            if __consumed == 0 {
                                // recoverable: ran out of input before
                                // a single key byte was consumed.
                                return ::core::result::Result::Ok(
                                    ::tinyklv::decoder::Packet::NeedMore(__acc)
                                );
                            }
                            // --------------------------------------------------
                            // unrecoverable: key bytes existed but could not be decoded. attempt finalize
                            // on whatever the partial holds; success surfaces as Ready, failure propagates
                            // the label outward to the Decoder (which owns input/checkpoint context)
                            // --------------------------------------------------
                            return match <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(__acc) {
                                Ok(v) => ::core::result::Result::Ok(
                                    ::tinyklv::decoder::Packet::Ready(v)
                                ),
                                Err(label) => ::core::result::Result::Err(label),
                            };
                        }
                    };
                    let len = match #len_decoder.parse_next(input) {
                        Ok(l) => l,
                        Err(_) => {
                            // --------------------------------------------------
                            // key parsed but len short: always
                            // truncation. rewind to iteration start so
                            // both key and len will be re-read once
                            // more bytes arrive.
                            // --------------------------------------------------
                            input.reset(&checkpoint_inner);
                            return ::core::result::Result::Ok(
                                ::tinyklv::decoder::Packet::NeedMore(__acc)
                            );
                        }
                    };

                    match Self::break_condition(key, len) {
                        ::tinyklv::BreakConditionType::Proceed => (),
                        ::tinyklv::BreakConditionType::Skip => {
                            // --------------------------------------------------
                            // not enough bytes for the value being
                            // skipped: rewind so the next iteration
                            // re-reads the same key/len once more
                            // bytes arrive, then surface the in-flight
                            // partial via NeedMore.
                            // --------------------------------------------------
                            if input.eof_offset() < len {
                                input.reset(&checkpoint_inner);
                                return ::core::result::Result::Ok(
                                    ::tinyklv::decoder::Packet::NeedMore(__acc)
                                );
                            }
                            // --------------------------------------------------
                            // pre-checked above; `take` should never fail here
                            // --------------------------------------------------
                            let _ = ::tinyklv::__export::winnow::token::take::<
                                usize,
                                #stream,
                                ::tinyklv::__export::winnow::error::ContextError,
                            >(len).parse_next(input);
                            continue;
                        }
                        ::tinyklv::BreakConditionType::Done => break,
                        ::tinyklv::BreakConditionType::Abort(_) => {
                            // --------------------------------------------------
                            // user-signalled abort. the abort's own
                            // ContextError is not preserved here - the
                            // codegen returns a static label and the
                            // Decoder wraps it with real input
                            // context. carrying the user's
                            // ContextError through the &'static str
                            // boundary is left as a follow-up.
                            // --------------------------------------------------
                            return match <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(__acc) {
                                Ok(v) => ::core::result::Result::Ok(
                                    ::tinyklv::decoder::Packet::Ready(v)
                                ),
                                Err(_) => ::core::result::Result::Err(
                                    "break_condition::abort"
                                ),
                            };
                        }
                        // future `BreakConditionType` variants behave as `Proceed`
                        _ => (),
                    }
                    #debug_key_val
                    #pre_check_gate
                    if input.eof_offset() < len {
                        // --------------------------------------------------
                        // value bytes incomplete for the current key.
                        // rewind, surface partial via NeedMore.
                        // --------------------------------------------------
                        input.reset(&checkpoint_inner);
                        return ::core::result::Result::Ok(
                            ::tinyklv::decoder::Packet::NeedMore(__acc)
                        );
                    }
                    // --------------------------------------------------
                    // extract the value subinput. pre-checked above, so
                    // the take cannot fail in practice; on the unlikely
                    // parser-internal error attempt finalize on the
                    // partial and surface its outcome.
                    // --------------------------------------------------
                    let mut subinput: <#stream as ::tinyklv::__export::winnow::stream::Stream>::Slice = match ::tinyklv::__export::winnow::token::take::<
                        usize,
                        #stream,
                        ::tinyklv::__export::winnow::error::ContextError,
                    >(len).parse_next(input) {
                        Ok(s) => s,
                        Err(_) => return match <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(__acc) {
                            Ok(v) => ::core::result::Result::Ok(
                                ::tinyklv::decoder::Packet::Ready(v)
                            ),
                            Err(label) => ::core::result::Result::Err(label),
                        },
                    };
                    // --------------------------------------------------
                    // field dispatch into the partial accumulator.
                    // unknown keys (when allowed) hit the `_ => ()` arm
                    // and are silently ignored after their bytes have
                    // already been consumed by the take above.
                    // --------------------------------------------------
                    match key {
                        #items_match
                        _ => (),
                    }
                }
                // --------------------------------------------------
                // BreakConditionType::Done dropped us out of the loop.
                // attempt finalize on whatever the partial holds.
                // success -> Ready; required-field failure -> propagate
                // the &'static str label outward (the Decoder will
                // wrap it with input/checkpoint context).
                // --------------------------------------------------
                match <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(__acc) {
                    Ok(v) => ::core::result::Result::Ok(
                        ::tinyklv::decoder::Packet::Ready(v)
                    ),
                    Err(label) => ::core::result::Result::Err(label),
                }
            }
        }
    }
}
