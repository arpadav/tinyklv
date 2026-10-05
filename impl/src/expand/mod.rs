#![allow(
    clippy::unwrap_used,
    reason = "proc macro, context check/panic is required"
)]
//! Token expansion entry point for the `#[derive(Klv)]` proc-macro
//!
//! Drives the full derive pipeline: parses the input into a [`MainContainer`],
//! checks for attribute errors, and emits the token streams. Whether decode and
//! encode are generated is decided by `decode_impl::gen_decode_impl` and
//! `encode_impl::gen_encode_impl`
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
use crate::{Ctxt, ast::attr::MainContainer};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// Drives the full `#[derive(Klv)]` expansion
///
/// Parses the derive input into a [`MainContainer`], validates all container
/// and field attributes, then emits whatever decode and encode implementations
/// `decode_impl::gen_decode_impl` and `encode_impl::gen_encode_impl` generate
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
    // impl encode
    // --------------------------------------------------
    if let Some(encode_impls) = encode_impl::gen_encode_impl(&cont) {
        expanded = quote! {
            #expanded
            #encode_impls
        }
    }
    Ok(expanded)
}
