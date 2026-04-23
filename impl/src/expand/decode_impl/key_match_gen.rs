// --------------------------------------------------
// local
// --------------------------------------------------
use super::constants;
use super::helpers;
use crate::ast::attr::MainField;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates a `|`-joined pattern of every field's `#[klv(key = ..)]` literal,
/// suitable for the RHS of a `matches!(key, #pattern)` expression.
///
/// Used by the pre-match unknown-key gate inside `decode_partial`: before
/// `take(len)` consumes the value bytes, we check whether `key` is one the
/// struct declared. If not, and the container carries `deny_unknown_keys`, we
/// bail with [`tinyklv::prelude::Progress::Malformed`] immediately - no bytes
/// wasted, and the error says "unknown key" rather than the downstream
/// "packet truncated" the old ordering would have produced when the declared
/// length overran remaining input.
///
/// # Degenerate case
///
/// A struct with zero klv-annotated fields has no known keys at all. We emit
/// `_ if false` which is a never-match pattern, so `matches!(key, _ if false)`
/// is always `false` and every key is treated as unknown. Under
/// `deny_unknown_keys` this rejects everything, which is the only sensible
/// behavior for a struct that declared no keys.
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
    let pattern = quote! { #(#keys)|* };
    // --------------------------------------------------
    // return the matches
    // --------------------------------------------------
    quote! {
        if !matches!(key, #pattern) {
            return ::core::result::Result::Err(
                concat!(
                    "invalid key (expected one of the keys defined on `",
                    stringify!(#name),
                    "`; to turn this off, remove `deny_unknown_keys`)",
                )
            );
        }
    }
}

/// Generates the tokens for matching the key/len's with fields and parsers
///
/// `#key => __acc.#name = #dec #optional_len_arg (&mut subinput).ok(),`
///
/// Where `subinput` is a sub-slice of `input` of the value's length,
/// designated by the stream after its key, and `__acc` is the in-flight
/// `XxxPartialPacket` being filled by the surrounding
/// `ResumePartial::resume_partial` body.
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
            // varlen - changes fn signature
            // --------------------------------------------------
            let varlen = attrs.var.as_ref().map(|v| v.value).unwrap_or(false); // <-- defaults to false
            let optional_len_arg = if varlen {
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
            match debug {
                true => {
                    let logger = constants::logger();
                    quote! {
                        #key => {
                            let val = #dec_tokens #optional_len_arg (&mut subinput);
                            #logger ("\t{}: {:?}", stringify!(#name), val);
                            __acc.#name = val.ok() #latebind_map .or(__acc.#name);
                        },
                    }
                }
                false => quote! {
                    #key => __acc.#name = #dec_tokens #optional_len_arg (&mut subinput).ok() #latebind_map .or(__acc.#name),
                },
            }
        });
    // --------------------------------------------------
    // return all match arms
    // --------------------------------------------------
    quote! { #(#arms)* }
}
