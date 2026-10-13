//! Proc-macro implementation crate for `#[derive(Klv)]`
//!
//! Drives the full derive pipeline for the [`tinyklv`] crate's `Klv` derive
//! macro. When a struct is annotated with `#[derive(Klv)]` and the appropriate
//! `#[klv(..)]` attribute, this crate:
//!
//! 1. Parses the derive input and all `#[klv(..)]` attributes into an AST
//!    representation (`ast` module)
//! 2. Accumulates diagnostics via [`Ctxt`] and emits compile errors for any
//!    invalid or missing annotations
//! 3. Conditionally generates `EncodeValue`/`EncodeFrame` and
//!    `DecodeValue`/`DecodePartial`/`ResumePartial`/`SeekSentinel` implementations
//!    depending on the container's framing and on which fields carry encoders
//!    and decoders (`expand` module). `DecodeFrame` is not generated: `tinyklv`'s
//!    blanket impl provides it for every `SeekSentinel` + `DecodeValue` type
//!
//! This crate is an implementation detail of `tinyklv` and is not intended to
//! be used directly
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[macro_use]
mod err;
mod ast;
mod ctxt;
mod expand;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{ast::symbol, ctxt::Ctxt, err::Error};

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{DeriveInput, parse_macro_input};

/// Entry point for the `#[derive(Klv)]` proc-macro
///
/// Parses the annotated struct into a [`syn::DeriveInput`], delegates the full
/// AST parsing and code-generation pipeline to [`expand::derive`], and converts
/// any [`syn::Error`] diagnostics into compile errors via
/// [`syn::Error::into_compile_error`]
///
/// The generated implementations depend on which `#[klv(..)]` attributes are
/// present. A missing encoder, on a field, on the framing attribute (the
/// `key(..)` + `len(..)` pair or the fused `keylen(..)` codec) or on the
/// `unknown(..)` codec of a struct whose `#[klv(unknown)]` field keeps its
/// frames, is a compile error unless the container sets
/// `allow_unimplemented_encode`, in which case no encode impl is generated.
/// Missing decoders, `unknown(..)`'s included, behave the same under
/// `allow_unimplemented_decode`, suppressing the decode impls
#[proc_macro_derive(Klv, attributes(klv))]
pub fn klv_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
