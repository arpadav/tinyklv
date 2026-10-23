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
//! `default` form (no `=` token) cannot be expressed via the macro. `parse_pnm_size` (a field's
//! `size(..)` group, delegated to [`SizeSpec`]) and `parse_pnm_header_size` (a `key(..)`/`len(..)`
//! `size(..)` group, delegated to [`SizeBytes`]) are hand-written too; the bare `implicit`,
//! `lengthed`, `skip_if_default` and `unknown` flags share `pnm_parse_flag`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    ast::{
        attr::{
            KeySpec,
            size::{SizeBytes, SizeSpec},
        },
        types::*,
    },
    symbol::*,
};
use tk_syn_macros::create_parser;

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::spanned::Spanned;

/// Parses a value of type `T` from a [`syn::meta::ParseNestedMeta`], accepting
/// the value either as raw tokens or wrapped in a [`syn::LitStr`]
///
/// A [`syn::LitStr`] value has its string contents parsed as `T` (allowing
/// `enc = "my::path"` in addition to `enc = my::path`); any other literal is rejected, naming
/// both forms. Otherwise the raw token stream is parsed directly as `T`
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
    let value = input.value()?;
    if value.peek(syn::LitStr) {
        return value.parse::<syn::LitStr>()?.parse();
    }
    if value.peek(syn::Lit) {
        return Err(value.error(err!(LiteralSetting(input.path))));
    }
    value.parse()
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

// encoder / decoder
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
// A frame header encoder, which takes no sigil
create_parser!(
    ENCODER: HeaderEncoder;
    pnm_parse_maybestr_header_encoder, pnm_parse_maybestr => syn::meta::ParseNestedMeta
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

// type / stream
create_parser!(
    TYPE: TypeType;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    STREAM: TypeType;
    nv_parse_maybestr => syn::MetaNameValue
);

// key (a literal or a `oneof(..)`) / the `value` of a `sentinel(..)` list
create_parser!(KEY: KeySpec; pnm);
create_parser!(SENTINEL_VALUE: syn::Lit; pnm);

// size (field size shape: `size(implicit, lengthed, exact = N | reserve = N)`; key/len byte
// count: `size(exact = N | reserve = N)`) and its inner keywords
// the `exact = N` / `reserve = N` byte counts (kept as `LitInt` so a conflict can be spanned)
create_parser!(EXACT: syn::LitInt; pnm);
create_parser!(RESERVE: syn::LitInt; pnm);

/// Parses a bare flag (a path, no `= value`) inside a list
///
/// Modelled on [`parse_pnm_default_value`]'s bare form: an `Err` from `value()` means the flag was
/// present with no value (the valid form); an `Ok` means the user wrote `<flag> = ..`, which is
/// rejected, as is a `<flag>(..)` list
///
/// # Arguments
///
/// * `input` - the nested meta entry, positioned at the flag
///
/// # Returns
///
/// The span of the flag
///
/// # Errors
///
/// The flag written with a value or a list
fn pnm_parse_flag(input: &syn::meta::ParseNestedMeta) -> syn::Result<proc_macro2::Span> {
    let flag = input.path.to_token_stream();
    match input.value() {
        Ok(_) => Err(input.error(format!("`{flag}` is a flag and takes no value"))),
        Err(_) if input.input.peek(syn::token::Paren) => Err(input.error(format!(
            "`{flag}` is a flag and takes no list: write `{flag}`"
        ))),
        Err(_) => Ok(input.path.span()),
    }
}

// the bare `implicit` / `lengthed` flags
create_parser!(
    IMPLICIT: proc_macro2::Span;
    parse_pnm_implicit, pnm_parse_flag => syn::meta::ParseNestedMeta
);
create_parser!(
    LENGTHED: proc_macro2::Span;
    parse_pnm_lengthed, pnm_parse_flag => syn::meta::ParseNestedMeta
);

/// Parses a field's [`SizeSpec`] from the `size(..)` group on a [`syn::meta::ParseNestedMeta`]
///
/// Defers straight to [`SizeSpec::try_from`], which recurses into the parenthesised group with
/// [`ParseNestedMeta::parse_nested_meta`] - so the inner `lengthed`/`exact`/`reserve` parse goes
/// through [`handle_unique_nested_meta_values!`](tk_syn_macros::handle_unique_nested_meta_values)
/// like every other list attribute, with no `syn::MetaList` re-materialisation
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

/// Parses a `key(..)` or `len(..)` byte count, [`SizeBytes`], from the `size(..)` group on a
/// [`syn::meta::ParseNestedMeta`]
///
/// The `key(..)`/`len(..)` counterpart of [`parse_pnm_size`]: defers to [`SizeBytes::try_from`],
/// which parses the inner `exact`/`reserve` the same way and has no `lengthed`
///
/// # Arguments
///
/// * `input` - the nested meta entry, positioned at the `size` keyword (before its `(..)`)
///
/// # Returns
///
/// [`None`] if the keyword is not [`SIZE`]; otherwise the parsed [`SizeBytes`] or a parse error
pub(crate) fn parse_pnm_header_size(
    input: &syn::meta::ParseNestedMeta,
) -> Option<syn::Result<SizeBytes>> {
    if input.path != SIZE {
        return None;
    }
    Some(SizeBytes::try_from(input))
}

// default
/// Parses a [`DefaultValue`] from a [`syn::meta::ParseNestedMeta`]
///
/// Accepts two forms on field attributes:
///
/// * bare `default` (no `=` token) -> [`DefaultValue::Call`], which codegen
///   lowers to `<T as ::core::default::Default>::default()`
/// * `default = <expr>` -> [`DefaultValue::Expr`], which codegen splices as-is
///
/// Not emitted via `create_parser!` because the bare form must yield
/// `Some(Ok(Call))` on a missing `=`, which the macro's helper can't express
///
/// # Arguments
///
/// * `input` - the nested meta entry, positioned at the `default` keyword
///
/// # Returns
///
/// * [`None`] if the keyword is not [`DEFAULT_VALUE`]
/// * `Some(Ok(..))` on a successful parse (both bare and name-value forms)
/// * `Some(Err(..))` if `default = <expr>` was written but `<expr>` failed
///   to parse as a [`syn::Expr`]
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

// latebind
create_parser!(
    LATEBIND: LatebindXcoder;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);

// a field's `break_on`, the path of its `fn(&value) -> BreakType`
create_parser!(
    BREAK_ON: syn::Path;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);

// a field's or a `default(..)`'s skip: the bare `skip_if_default` flag, or the path of its
// `skip_if` fn
create_parser!(
    SKIP_IF_DEFAULT: proc_macro2::Span;
    parse_pnm_skip_if_default, pnm_parse_flag => syn::meta::ParseNestedMeta
);
create_parser!(
    SKIP_IF: syn::Path;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);

// the bare `unknown` flag of the field that keeps the frames whose key no field declares
create_parser!(
    UNKNOWN_FRAMES: proc_macro2::Span;
    parse_pnm_unknown, pnm_parse_flag => syn::meta::ParseNestedMeta
);
create_parser!(
    LATEBIND: LatebindXcoder;
    nv_parse_maybestr => syn::MetaNameValue
);
