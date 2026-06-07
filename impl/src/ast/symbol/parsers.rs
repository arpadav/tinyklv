//! Symbol-specific parser functions for `#[klv(..)]` attribute values
//!
//! Contains the two "maybe-litstr" helper parsers shared by all generated
//! xcoder parsers, plus the [`create_parser!`]-generated parser functions for
//! every recognized `#[klv(..)]` keyword. Each generated function follows the
//! `Option<syn::Result<T>>` protocol expected by
//! [`tk_syn_macros::handle_unique_nested_meta_values!`]:
//!
//! * `None` - the keyword was not matched (this entry is not for this parser)
//! * `Some(Ok(T))` - keyword matched and value parsed successfully
//! * `Some(Err(e))` - keyword matched but value was malformed
//!
//! The `parse_pnm_default_value` function is hand-written because the bare
//! `default` form (no `=` token) cannot be expressed via the macro
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use tk_syn_macros::create_parser;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::size::SizeSpec;
use crate::ast::types::*;
use crate::symbol::*;

/// Parses a value of type `T` from a [`syn::meta::ParseNestedMeta`], accepting
/// the value either as raw tokens or wrapped in a [`syn::LitStr`]
///
/// First attempts to parse the value stream as a [`syn::LitStr`]; if that
/// succeeds, re-parses the string contents as `T` (allowing `enc = "my::path"`
/// in addition to `enc = my::path`). If the value is not a string literal,
/// falls back to parsing the raw token stream directly as `T`
///
/// # Arguments
///
/// * `input` - The nested meta entry whose value is to be parsed
///
/// # Returns
///
/// `Ok(T)` on a successful parse, or a [`syn::Error`] if the value is present
/// but cannot be parsed as `T` in either form
fn pnm_parse_maybestr<T: syn::parse::Parse>(input: &syn::meta::ParseNestedMeta) -> syn::Result<T> {
    match input.value() {
        Ok(value) => match value.parse::<syn::LitStr>() {
            Ok(litstr) => litstr.parse(),
            Err(_) => value.parse(),
        },
        Err(err) => Err(err),
    }
}

/// Parses a value of type `T` from a [`syn::MetaNameValue`], accepting
/// the value either as raw tokens or wrapped in a [`syn::LitStr`]
///
/// First attempts to parse the value expression as a [`syn::LitStr`]; if that
/// succeeds, re-parses the string contents as `T`. Otherwise falls back to
/// parsing the raw token stream of the value expression directly as `T`
///
/// # Arguments
///
/// * `input` - The name-value meta entry whose value is to be parsed
///
/// # Returns
///
/// `Ok(T)` on a successful parse, or a [`syn::Error`] if the value cannot
/// be parsed as `T` in either form
fn nv_parse_maybestr<T: syn::parse::Parse>(input: &syn::MetaNameValue) -> syn::Result<T> {
    let value_tokens = input.value.to_token_stream();
    match syn::parse2::<syn::LitStr>(value_tokens.clone()) {
        Ok(litstr) => litstr.parse(),
        Err(_) => T::parse.parse2(value_tokens),
    }
}

// --------------------------------------------------
// encoder / decoder
// --------------------------------------------------
// A siguled xcoder, for different encoding fn signatures
// using &
create_parser!(
    ENCODER: SiguledXcoder;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    ENCODER: SiguledXcoder;
    nv_parse_maybestr => syn::MetaNameValue
);
// A non siguled xcoder, for different encoding fn signatures
create_parser!(
    ENCODER: XcoderType;
    pnm_parse_maybestr_encoder_no_sigil, pnm_parse_maybestr => syn::meta::ParseNestedMeta
);

// A non siguled xcoder, for different decoding fn signatures
create_parser!(
    DECODER: XcoderType;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    DECODER: XcoderType;
    nv_parse_maybestr => syn::MetaNameValue
);

// --------------------------------------------------
// type / stream
// --------------------------------------------------
create_parser!(
    TYPE: TypeType;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    STREAM: TypeType;
    nv_parse_maybestr => syn::MetaNameValue
);

// --------------------------------------------------
// key / sentinel
// --------------------------------------------------
create_parser!(KEY: syn::Lit; pnm);
create_parser!(SENTINEL: syn::Lit; nv);

// --------------------------------------------------
// size (key/len/field size shape: `size(var, exact = N | hint = N)`) and its inner keywords
// --------------------------------------------------
// the `exact = N` / `hint = N` byte counts (kept as `LitInt` so a conflict can be spanned)
create_parser!(EXACT: syn::LitInt; pnm);
create_parser!(HINT: syn::LitInt; pnm);

/// Parses the bare `var` flag inside a `size(..)` list (a path, no `= value`)
///
/// Modelled on [`parse_pnm_default_value`]'s bare form: an `Err` from `value()` means the flag was
/// present with no value (the valid form); an `Ok` means the user wrote `var = ..`, which is rejected
///
/// # Arguments
///
/// * `input` - the nested meta entry, positioned at the `var` keyword
///
/// # Returns
///
/// [`None`] if the keyword is not [`VAR`]; `Some(Ok(()))` for a bare `var`; `Some(Err(..))` otherwise
pub(crate) fn parse_pnm_var(input: &syn::meta::ParseNestedMeta) -> Option<syn::Result<()>> {
    if input.path != VAR {
        return None;
    }
    Some(match input.value() {
        Ok(_) => Err(input.error("`var` is a flag and takes no value")),
        Err(_) => Ok(()),
    })
}

/// Parses a [`SizeSpec`] from the `size(..)` group on a [`syn::meta::ParseNestedMeta`]
///
/// Defers straight to [`SizeSpec::try_from`], which recurses into the parenthesised group with
/// [`ParseNestedMeta::parse_nested_meta`] - so the inner `var`/`exact`/`hint` parse goes through
/// [`handle_unique_nested_meta_values!`](tk_syn_macros::handle_unique_nested_meta_values) like every
/// other list attribute, with no `syn::MetaList` re-materialisation
///
/// # Arguments
///
/// * `input` - the nested meta entry, positioned at the `size` keyword (before its `(..)`)
///
/// # Returns
///
/// [`None`] if the keyword is not [`SIZE`]; otherwise the parsed [`SizeSpec`] or a parse error
///
/// [`ParseNestedMeta::parse_nested_meta`]: syn::meta::ParseNestedMeta::parse_nested_meta
pub(crate) fn parse_pnm_size(input: &syn::meta::ParseNestedMeta) -> Option<syn::Result<SizeSpec>> {
    if input.path != SIZE {
        return None;
    }
    Some(SizeSpec::try_from(input))
}

// --------------------------------------------------
// default
// --------------------------------------------------
/// Parses a [`DefaultValue`] from a [`syn::meta::ParseNestedMeta`]
///
/// Accepts two forms on field attributes:
///
/// * bare `default` (no `=` token) -> [`DefaultValue::Call`], which codegen
///   lowers to `<T as ::core::default::Default>::default()`
/// * `default = <expr>` -> [`DefaultValue::Expr`], which codegen splices as-is
///
/// Returns:
///
/// * [`None`] if the keyword is not [`DEFAULT_VALUE`]
/// * [`Some(Ok(..))`] on a successful parse (both bare and name-value forms)
/// * [`Some(Err(..))`] if `default = <expr>` was written but `<expr>` failed
///   to parse as a [`syn::Expr`]
///
/// Not emitted via `create_parser!` because the bare form must yield
/// `Some(Ok(Call))` on a missing `=`, which the macro's helper can't express
pub(crate) fn parse_pnm_default_value(
    input: &syn::meta::ParseNestedMeta,
) -> Option<syn::Result<DefaultValue>> {
    if input.path != DEFAULT_VALUE {
        return None;
    }
    Some(match input.value() {
        Ok(value) => value.parse::<syn::Expr>().map(DefaultValue::Expr),
        Err(_) => Ok(DefaultValue::Call),
    })
}

// --------------------------------------------------
// latebind
// --------------------------------------------------
create_parser!(
    LATEBIND: LatebindXcoder;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    LATEBIND: LatebindXcoder;
    nv_parse_maybestr => syn::MetaNameValue
);
