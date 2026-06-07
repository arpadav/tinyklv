//! Code generation for key-dispatch match arms and unknown-key pre-checks
//!
//! Provides two code-generation helpers used by `decode_partial_gen`:
//!
//! * [`gen_known_keys_check`] - emits a `matches!` guard that rejects unknown
//!   keys before value bytes are consumed, when `deny_unknown_keys` is set
//! * [`gen_items_match`] - emits the per-field match arms that route each key
//!   to its decoder and write the result into the partial-packet accumulator
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::helpers;
use crate::ast::attr::MainField;
use crate::ast::attr::size::SizeSpec;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates a `|`-joined pattern of every field's `#[klv(key = ..)]` literal,
/// suitable for the RHS of a `matches!(key, #pattern)` expression
///
/// Used by the pre-match unknown-key gate inside `decode_partial`: before
/// `take(len)` consumes the value bytes, we check whether `key` is one the
/// struct declared. If not, and the container carries `deny_unknown_keys`, we
/// bail with [`tinyklv::prelude::Packet::Malformed`] immediately - no bytes
/// wasted, and the error says "unknown key" rather than the downstream
/// "packet truncated" the old ordering would have produced when the declared
/// length overran remaining input
///
/// # Degenerate case
///
/// A struct with zero klv-annotated fields has no known keys at all. We emit
/// `_ if false` which is a never-match pattern, so `matches!(key, _ if false)`
/// is always `false` and every key is treated as unknown. Under
/// `deny_unknown_keys` this rejects everything, which is the only sensible
/// behavior for a struct that declared no keys
///
/// # Emission shape
///
/// ```text
/// 0x01 | 0x02 | 0x03
/// ```
///
/// or for enum-like keys:
///
/// ```text
/// Key::Foo | Key::Bar
/// ```
pub(super) fn gen_known_keys_check(
    name: &syn::Ident,
    fields: &[MainField],
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // wrap the known-keys pattern in the streaming gate, which returns the `&'static str` label
    // expected by `resume_partial`'s `Result<_, &'static str>`
    // --------------------------------------------------
    let pattern = gen_known_keys_pattern(fields);
    let message = gen_unknown_key_message(name);
    quote! {
        if !matches!(key, #pattern) {
            return ::core::result::Result::Err(#message);
        }
    }
}

/// Generates the `deny_unknown_keys` rejection message as a `concat!` expression
///
/// Both gates - the streaming [`gen_known_keys_check`] (returning a `&'static str`) and the
/// one-shot direct gate (wrapping it in a `ContextError` via `labeled_error`) - wrap the same key
/// pattern with this identical message, differing only in the error type. Generated here once so
/// the wording cannot drift between the two decode paths
///
/// # Arguments
///
/// * `name` - The container ident, named in the message so the user knows which struct's keys to check
///
/// # Returns
///
/// A `concat!(..)` token expression evaluating to a `&'static str`
pub(super) fn gen_unknown_key_message(name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        concat!(
            "invalid key (expected one of the keys defined on `",
            stringify!(#name),
            "`; to turn this off, remove `deny_unknown_keys`)",
        )
    }
}

/// Generates the `|`-joined alternation of every field's `#[klv(key = ..)]` literal
///
/// The pattern feeds a `matches!(key, #pattern)` guard. A struct with no KLV fields yields the
/// never-match `_ if false`, so every key is treated as unknown. Shared by the streaming
/// [`gen_known_keys_check`] (which returns a `&'static str` label) and the one-shot direct decoder
/// (which wraps it to return a `ContextError`), so the key set lives in one place
///
/// # Arguments
///
/// * `fields` - All fields of the container; non-KLV fields contribute no key
///
/// # Returns
///
/// The pattern token stream (`0x01 | 0x02 | ..` or `_ if false`)
pub(super) fn gen_known_keys_pattern(fields: &[MainField]) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // collect each klv field's declared key expression
    // --------------------------------------------------
    let keys: Vec<_> = fields
        .iter()
        .filter_map(|f| f.attrs.as_ref().map(|a| a.key.clone()))
        .collect();
    // --------------------------------------------------
    // degenerate: no klv fields -> never-match pattern
    // --------------------------------------------------
    if keys.is_empty() {
        return quote! { _ if false };
    }
    // --------------------------------------------------
    // splice keys with `|` separators to form a pattern alternation
    // --------------------------------------------------
    quote! { #(#keys)|* }
}

/// Generates match arms that dispatch a parsed key to its per-field decoder
///
/// For each field that carries a `#[klv(..)]` annotation this function emits
/// one match arm of the form:
///
/// ```text
/// #key => __acc.#name = #dec #optional_len_arg (&mut subinput).ok() #latebind_map .or(__acc.#name),
/// ```
///
/// Where `subinput` is a sub-slice of `input` of exactly `len` bytes (already
/// consumed by the surrounding `take`), and `__acc` is the in-flight
/// `XxxPartialPacket` accumulator. The `.ok().or(__acc.#name)` merge is LAST-WINS:
/// since `Some(new).or(prev) == Some(new)`, a duplicate key that decodes
/// successfully overwrites the earlier value, while a duplicate key that fails to
/// decode keeps the prior valid one. When the `debug` flag is set each arm additionally logs the key
/// name and decoded value via the `logger()` macro
///
/// # Arguments
///
/// * `fields` - All fields of the container; those without KLV annotations are skipped
/// * `stream` - The stream type, used to qualify the `DecodeValue` fallback trait call
/// * `debug` - When `true`, emit per-field debug logging inside each match arm
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing one match arm per annotated field,
/// ready to be spliced into the `match key { .. }` block in `resume_partial`
pub(super) fn gen_items_match(
    fields: &[MainField],
    stream: &syn::Type,
    debug: bool,
) -> proc_macro2::TokenStream {
    let arms = fields
        .iter()
        .filter_map(|f| f.attrs.as_ref().map(|attr| (&f.name, f.ty, attr)))
        .map(|(name, ty, attrs)| {
            // --------------------------------------------------
            // key - literal
            // --------------------------------------------------
            let key = &attrs.key;
            // --------------------------------------------------
            // decoder - function/macro
            // --------------------------------------------------
            let dec_tokens: proc_macro2::TokenStream = if attrs.fallback_dec {
                // --------------------------------------------------
                // `fallback_dec` indicates using DecodeValue impl
                // --------------------------------------------------
                let t = helpers::unwrap_option_type(ty).unwrap_or(ty);
                quote! { <#t as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
            } else {
                // --------------------------------------------------
                // otherwise, use explicit defined decoder
                // --------------------------------------------------
                #[allow(clippy::unwrap_used, reason = "`gen_decode_impl` call ensures that `attrs.dec` is `Some`")]
                let dec = attrs.dec.as_ref().unwrap();
                quote! { #dec }
            };
            // --------------------------------------------------
            // size - a `var` size makes the decoder take the runtime `len`; a reserve-only size or
            // an omitted one reads without it (defaults to no `len` arg)
            // --------------------------------------------------
            let takes_len = attrs.size.is_some_and(SizeSpec::takes_len);
            let optional_len_arg = if takes_len {
                quote! { (len) }
            } else {
                quote! {}
            };
            // --------------------------------------------------
            // latebind - optional transform post-decode
            // --------------------------------------------------
            let latebind_map = match attrs.latebind.as_ref() {
                Some(lb) => {
                    let inner = &lb.inner;
                    if lb.is_mut {
                        // --------------------------------------------------
                        // mutating transform
                        // --------------------------------------------------
                        quote! { .map(|mut __v| { #inner(&mut __v); __v }) }
                    } else {
                        // --------------------------------------------------
                        // consuming transform
                        // --------------------------------------------------
                        quote! { .map(#inner) }
                    }
                }
                None => quote! {},
            };
            // --------------------------------------------------
            // field assignment with optional logging
            // --------------------------------------------------
            if debug {
                let logger = crate::expand::logger::debug_logger();
                quote! {
                    #key => {
                        let val = #dec_tokens #optional_len_arg (&mut subinput);
                        #logger ("\t{}: {:?}", stringify!(#name), val);
                        __acc.#name = val.ok() #latebind_map .or(__acc.#name);
                    },
                }
            } else { quote! {
                #key => __acc.#name = #dec_tokens #optional_len_arg (&mut subinput).ok() #latebind_map .or(__acc.#name),
            } }
        });
    // --------------------------------------------------
    // return all match arms
    // --------------------------------------------------
    quote! { #(#arms)* }
}
