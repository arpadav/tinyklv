#![allow(
    clippy::unwrap_used,
    reason = "proc macro, context check/panic is required"
)]
//! Token expansion entry point for the `#[derive(Klv)]` proc-macro
//!
//! Drives the full derive pipeline: parses the input into a [`MainContainer`],
//! checks for attribute errors, determines whether encode and/or decode
//! implementations can be generated based on the presence of encoders/decoders
//! on each field, and emits the corresponding token streams
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod decode_impl;
mod encode_impl;
pub(crate) mod helpers;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Ctxt;
use crate::ast::attr::MainContainer;

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// Drives the full `#[derive(Klv)]` expansion
///
/// Parses the derive input into a [`MainContainer`], validates all container
/// and field attributes, then conditionally emits `Decode`/`DecodeValue` and
/// `Encode`/`EncodeValue` implementations based on whether every field carries
/// the corresponding encoder or decoder. A partial set (some fields have
/// encoders, some do not) suppresses the implementation for that direction
/// entirely
///
/// # Arguments
///
/// * `input` - The raw `syn::DeriveInput` produced by the proc-macro harness
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
    // check to see if all encoders/decoders exist
    // --------------------------------------------------
    let mut all_encoders_exist = true;
    let mut all_decoders_exist = true;
    let mut any_encoders_exist = false;
    let mut any_decoders_exist = false;
    for f in cont.data.iter().filter_map(|f| f.attrs.as_ref()) {
        any_encoders_exist |= f.enc.is_some();
        any_decoders_exist |= f.dec.is_some();
        all_encoders_exist &= f.enc.is_some();
        all_decoders_exist &= f.dec.is_some();
    }
    all_encoders_exist &= any_encoders_exist;
    all_decoders_exist &= any_decoders_exist;
    // --------------------------------------------------
    // init the output token stream
    // --------------------------------------------------
    let mut expanded = quote! {};
    // --------------------------------------------------
    // impl decode
    // --------------------------------------------------
    if let (Some(key_dec), Some(len_dec), true) = (
        cont.attrs.key.dec.as_ref(),
        cont.attrs.len.dec.as_ref(),
        all_decoders_exist,
    ) {
        let decode_impls = decode_impl::gen_decode_impl(&cont, key_dec, len_dec);
        expanded = quote! {
            #expanded
            #decode_impls
        }
    }
    // --------------------------------------------------
    // impl encode
    // --------------------------------------------------
    if let (Some(key_enc), Some(len_enc), true) = (
        cont.attrs.key.enc.as_ref(),
        cont.attrs.len.enc.as_ref(),
        all_encoders_exist,
    ) {
        let encode_impls = encode_impl::gen_encode_impl(&cont, key_enc, len_enc);
        expanded = quote! {
            #expanded
            #encode_impls
        }
    }
    Ok(expanded)
}
