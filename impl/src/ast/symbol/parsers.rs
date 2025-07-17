// --------------------------------------------------
// external
// --------------------------------------------------
use tk_syn_macros::create_parser;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Length;
use crate::symbol::*;
use crate::ast::types::*;

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
create_parser!(
    ENCODER: XcoderType;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    DECODER: XcoderType;
    pnm_parse_maybestr => syn::meta::ParseNestedMeta
);
create_parser!(
    ENCODER: XcoderType;
    nv_parse_maybestr => syn::MetaNameValue
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
// variable length x 2
// --------------------------------------------------
pub(crate) fn parse_pnm_length(input: &syn::meta::ParseNestedMeta) -> Option<syn::Result<Length>> {
    if input.path != LENGTH {
        return None;
    }
    match input.value() {
        Ok(value) => {
            // --------------------------------------------------
            // try parse as integer
            // --------------------------------------------------
            if let Ok(litint) = value.parse::<syn::LitInt>() {
                let int: isize = match litint.base10_parse() {
                    Ok(int) => int,
                    Err(err) => return Some(Err(err)),
                };
                // --------------------------------------------------
                // if negative, return error
                // --------------------------------------------------
                if int < 0 {
                    return Some(Err(syn::Error::new_spanned(
                        litint,
                        err!(ExpectedLengthInField(int)),
                    )));
                }
                return Some(Ok(Length::Fixed(litint)));
            }
            // --------------------------------------------------
            // try to parse as path
            // --------------------------------------------------
            if let Ok(x) = value.parse::<syn::Path>() {
                // --------------------------------------------------
                // if not designated `VARIABLE_LENGTH`, return error
                // --------------------------------------------------
                if x != VARIABLE_LENGTH {
                    return Some(Err(syn::Error::new_spanned(
                        &x,
                        err!(ExpectedLengthInField(x)),
                    )));
                }
                return Some(Ok(Length::Variable(x)));
            }
            // --------------------------------------------------
            // can not parse correctly, return an error
            // --------------------------------------------------
            return Some(Err(syn::Error::new_spanned(
                &input.path,
                err!(ExpectedLengthInField(@String value.to_string())),
            )))
        },
        // --------------------------------------------------
        // failed to get value, return error
        // --------------------------------------------------
        Err(err) => Some(Err(err)),
    }
}

create_parser!(INITIAL_VALUE: syn::Expr; pnm);