#![allow(
    clippy::unwrap_used,
    reason = "proc macro, context check/panic is required"
)]
//! Token expansion entry point for the `#[derive(Klv)]` proc-macro
//!
//! Drives the full derive pipeline: parses the input into a [`MainContainer`],
//! checks for attribute errors, and emits the token streams. Whether decode and
//! encode are generated is decided by `decode_impl::gen_decode_impl` and
//! `encode_impl::gen_encode_impl`; a width no 32-bit `usize` holds gets a compile-time assert
//! ([`usize_asserts`])
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod decode_impl;
mod encode_impl;
mod helpers;
pub(crate) mod logger;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{Ctxt, ast::attr::MainContainer};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The first byte count past `u32::MAX`: the smallest `size(exact = N)` a 32-bit target's `usize`
/// cannot hold, and the exclusive bound on a four-byte length slot's body
#[cfg(test)]
const U32_PLUS_ONE_U64: u64 = u32::MAX as u64 + 1;

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
    // init the output token stream with the width asserts, if any
    // --------------------------------------------------
    let mut expanded = usize_asserts(&cont);
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

/// Emits a compile-time assert for each `size(exact = N)` on a field or on a `len(..)`, the
/// sentinel's own included, whose `N` is more than `u32::MAX`, so a target whose `usize` cannot
/// hold `N` fails to compile rather than wrapping the `usize` literals the derive emits for it
///
/// A proc macro runs on the host and cannot see the target, so the assert compares `N` with the
/// `usize::MAX` of the target the generated code compiles for: it holds on a 64-bit target and
/// fails on a 32-bit one, the narrowest that has `std`, which holds every smaller `N`
///
/// ```text
/// const _: () = ::core::assert!(Nu64 <= usize::MAX as u64, "`size(exact = N)` is more bytes ..");
/// ```
///
/// # Arguments
///
/// * `input` - The container, whose fields' and lengths' widths are checked
///
/// # Returns
///
/// The asserts, each located at its field's `size(..)`, the lengths' at the derive
fn usize_asserts(input: &MainContainer) -> TokenStream {
    // --------------------------------------------------
    // each field's exact width, at its `size(..)`, and each length's
    // --------------------------------------------------
    let fields = input.klv_keys().filter_map(|key| {
        let size = key.codec.size.as_ref()?;
        let at = proc_macro2::Span::call_site().located_at(size.span());
        Some((size.exact_width()?, at))
    });
    let lens = input
        .attrs
        .mode
        .lens()
        .filter_map(|len| len.size.exact_width())
        .map(|width| (width, proc_macro2::Span::call_site()));
    // --------------------------------------------------
    // an assert for each width a 32-bit `usize` cannot hold
    // --------------------------------------------------
    let asserts = fields
        .chain(lens)
        .filter(|(width, _)| u32::try_from(*width).is_err())
        .filter_map(|(width, at)| {
            let bound = u64::try_from(width).ok()?;
            let message =
                format!("`size(exact = {width})` is more bytes than this target's `usize` holds");
            Some(quote_spanned! {at=>
                const _: () = ::core::assert!(
                    #bound <= ::core::primitive::usize::MAX as ::core::primitive::u64,
                    #message,
                );
            })
        });
    quote! { #(#asserts)* }
}

#[cfg(test)]
mod tests {
    use super::{U32_PLUS_ONE_U64, usize_asserts};
    use crate::ast::attr::MainContainer;

    #[test]
    /// Verifies a `size(exact = N)` gets its `usize` assert only when `N` is more than
    /// `u32::MAX`, which a 32-bit target's `usize` cannot hold, on a field or a sentinel's `len`
    fn only_a_width_beyond_u32_gets_a_usize_assert() {
        let widest = proc_macro2::Literal::u32_unsuffixed(u32::MAX);
        let too_wide = proc_macro2::Literal::u64_unsuffixed(U32_PLUS_ONE_U64);
        let input: syn::DeriveInput = syn::parse_quote! {
            #[klv(
                sentinel(value = b"S", len(dec = decb::u8_as_usize, size(exact = #too_wide))),
                key(dec = decb::u8),
                len(dec = decb::u8_as_usize),
                allow_unimplemented_encode,
            )]
            struct Reading {
                #[klv(key = 0x01, dec = octets, size(lengthed, exact = #widest))]
                widest: Vec<u8>,
                #[klv(key = 0x02, dec = octets, size(lengthed, exact = #too_wide))]
                too_wide: Vec<u8>,
            }
        };
        let container = MainContainer::parsed(&input).expect("the input parses");
        let asserts = usize_asserts(&container).to_string();
        assert_eq!(asserts.matches("const _").count(), 2, "{asserts}");
        assert!(
            asserts.contains(&format!("{U32_PLUS_ONE_U64}u64")),
            "{asserts}"
        );
    }
}
