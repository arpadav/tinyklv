#![allow(
    clippy::unwrap_used,
    reason = "proc macro, context check/panic is required"
)]
//! Token expansion entry point for the `#[derive(Klv)]` proc-macro
//!
//! Drives the full derive pipeline: parses the input into a [`MainContainer`],
//! checks for attribute errors, decides whether the encode implementation can be
//! generated, and emits the token streams. Whether decode is generated is decided
//! by `decode_impl::gen_decode_impl`
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod decode_impl;
mod encode_impl;
pub(crate) mod helpers;
pub(crate) mod logger;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    Ctxt,
    ast::attr::{MainContainer, container::ContainerMode},
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// Drives the full `#[derive(Klv)]` expansion
///
/// Parses the derive input into a [`MainContainer`], validates all container
/// and field attributes, then emits whatever decode implementations
/// `decode_impl::gen_decode_impl` generates. The `EncodeValue` impl (and `EncodeFrame`
/// when a sentinel is declared) is emitted only for a separate `key(..)` + `len(..)` container with both
/// encoders whose every `#[klv]` field carries an encoder, and at least one does;
/// a partial set (some fields have encoders, some do not) suppresses it entirely
///
/// # Arguments
///
/// * `input` - The raw `syn::DeriveInput` produced by the proc-macro harness
///
/// # Returns
///
/// The token stream of every generated impl, or the accumulated attribute errors as a
/// [`syn::Error`] when the container fails to parse or validate
pub fn derive(input: &syn::DeriveInput) -> syn::Result<TokenStream> {
    // --------------------------------------------------
    // create a new error context
    // --------------------------------------------------
    let cx = Ctxt::new();
    // --------------------------------------------------
    // get the parsed container for Klv derive
    // --------------------------------------------------
    let Some(cont) = MainContainer::from_ast(&cx, input) else {
        return Err(cx.check().unwrap_err());
    };
    // --------------------------------------------------
    // check for errors
    // --------------------------------------------------
    cx.check()?;
    // --------------------------------------------------
    // check to see if all encoders exist
    // --------------------------------------------------
    let mut all_encoders_exist = true;
    let mut any_encoders_exist = false;
    for f in cont.data.iter().filter_map(|f| f.attrs.as_ref()) {
        any_encoders_exist |= f.enc.is_some();
        all_encoders_exist &= f.enc.is_some();
    }
    all_encoders_exist &= any_encoders_exist;
    // --------------------------------------------------
    // init the output token stream
    // --------------------------------------------------
    let mut expanded = quote! {};
    // --------------------------------------------------
    // impl decode
    // --------------------------------------------------
    if let Some(decode_impls) = decode_impl::gen_decode_impl(&cont) {
        expanded = quote! {
            #expanded
            #decode_impls
        }
    }
    // --------------------------------------------------
    // impl encode: separate key + len codecs only; a fused container generates decode only
    // --------------------------------------------------
    if let ContainerMode::Separate { key, len } = &cont.attrs.mode
        && let (Some(key_enc), Some(len_enc), true) =
            (key.enc.as_ref(), len.enc.as_ref(), all_encoders_exist)
    {
        let encode_impls =
            encode_impl::gen_encode_impl(&cont, key_enc, len_enc, key.size, len.size);
        expanded = quote! {
            #expanded
            #encode_impls
        }
    }
    Ok(expanded)
}
