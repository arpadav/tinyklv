// --------------------------------------------------
// external
// --------------------------------------------------
use tk_syn_macros::create_parser;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::types::*;
use crate::symbol::*;

/// Attempts to parse a type `T` from a [`syn::meta::ParseNestedMeta`]
///
/// If the type `T` can be surrounded with quotes (e.g. parse the contents
/// within the quotes of a [`syn::LitStr`], rather than trying to parse the
/// [`syn::LitStr`] as a type `T`), set `maybe_litstr` to `true`.
///
/// Otherwise, set `maybe_litstr` to `false`. (This is usually the case for
/// parsing literals, since you don't want to parse the contents within quotes
/// as a lit, instead, just parse the lit itself.)
fn pnm_parse_maybestr<T: syn::parse::Parse>(input: &syn::meta::ParseNestedMeta) -> syn::Result<T> {
    match input.value() {
        Ok(value) => match value.parse::<syn::LitStr>() {
            Ok(litstr) => litstr.parse(),
            Err(_) => value.parse(),
        },
        Err(err) => Err(err),
    }
}

/// Attempts to parse a type `T` from a [`syn::MetaNameValue`]
///
/// If the type `T` can be surrounded with quotes (e.g. parse the contents
/// within the quotes of a [`syn::LitStr`], rather than trying to parse the
/// [`syn::LitStr`] as a type `T`), set `maybe_litstr` to `true`.
///
/// Otherwise, set `maybe_litstr` to `false`. (This is usually the case for
/// parsing literals, since you don't want to parse the contents within quotes
/// as a lit, instead, just parse the lit itself.)
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
// variable length
// --------------------------------------------------
create_parser!(VARIABLE_LENGTH: syn::LitBool; pnm);

// --------------------------------------------------
// init
// --------------------------------------------------
create_parser!(INITIAL_VALUE: syn::Expr; pnm);

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
