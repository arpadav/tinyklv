//! Proc-macro implementation crate for `#[derive(Klv)]`
//!
//! Drives the full derive pipeline for the [`tinyklv`] crate's `Klv` derive
//! macro. When a struct is annotated with `#[derive(Klv)]` and the appropriate
//! `#[klv(..)]` attribute, this crate:
//!
//! 1. Parses the derive input and all `#[klv(..)]` attributes into an AST
//!    representation (`ast` module)
//! 2. Accumulates diagnostics via [`Ctxt`] and emits compile errors for any
//!    invalid or missing annotations
//! 3. Conditionally generates `Encode`/`EncodeValue`/`EncodeFrame` and
//!    `Decode`/`DecodeValue`/`DecodeFrame` implementations depending on
//!    which fields carry encoders and decoders (`expand` module)
//!
//! This crate is an implementation detail of `tinyklv` and is not intended to
//! be used directly
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[macro_use]
mod err;
mod ast;
mod ctxt;
mod expand;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ctxt::Ctxt;

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{DeriveInput, parse_macro_input};
use thiserror::Error;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The crate name referenced in user-facing error messages
const CRATE_NAME: &str = "tinyklv";

/// The derive macro name referenced in user-facing error messages (e.g. `#[derive(Klv)]`)
const DERIVE_NAME: &str = "Klv";

/// The attribute name used on containers and fields (e.g. `#[klv(..)]`)
const ATTR_NAME: &str = "klv";

/// Entry point for the `#[derive(Klv)]` proc-macro
///
/// Parses the annotated struct into a [`syn::DeriveInput`], delegates the full
/// AST parsing and code-generation pipeline to [`expand::derive`], and converts
/// any [`syn::Error`] diagnostics into compile errors via
/// [`syn::Error::into_compile_error`]
///
/// The generated implementations depend on which `#[klv(..)]` attributes are
/// present: containers without an encoder on every field produce no encode impl,
/// and similarly for decoders
#[proc_macro_derive(Klv, attributes(klv))]
pub fn klv_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// All diagnostic errors that the `#[derive(Klv)]` proc-macro can emit
///
/// Each variant corresponds to one class of user mistake in the `#[klv(..)]`
/// annotation or the shape of the annotated struct. Variants are formatted via
/// [`thiserror::Error`] and displayed directly in compiler diagnostics. The
/// [`crate::err!`] macro converts a variant into a [`std::borrow::Cow<str>`]
/// suitable for passing to [`crate::Ctxt::error_spanned_by`]
#[derive(Debug, Error)]
enum Error {
    // --------------------------------------------------
    // container parsing
    // --------------------------------------------------
    #[error("\
        {c} does not support #[derive({d})] for {0}s.",
        c = CRATE_NAME,
        d = DERIVE_NAME,
    )]
    UnsupportedContainer(String),

    #[error("\
        {c} does not support #[derive({d})] for structs with unnamed fields (tuple-structs).",
        c = CRATE_NAME,
        d = DERIVE_NAME,
    )]
    UnsupportedUnnamedStructs,

    #[error("\
        {c} does not support #[derive({d})] for unit-structs.",
        c = CRATE_NAME,
        d = DERIVE_NAME,
    )]
    UnsupportedUnitStructs,

    // --------------------------------------------------
    // container attributes
    // --------------------------------------------------
    #[error("\
        Unknown attribute: `{0}`.
Expected {s}.",
        s = symbol::CONT_SYMBOLS,
    )]
    UnknownContainerAttribute(String),

    #[error("\
        Unknown list attribute: `{0}(..)`.
Options include {s}.",
        s = symbol::CONT_LIST_SYMBOLS,
    )]
    UnknownContainerListAttribute(String),

    #[error("\
        Unknown name-value attribute: `{0} = <value>`.
Options include {s}.",
        s = symbol::CONT_NV_SYMBOLS,
    )]
    UnknownContainerNameValueAttribute(String),

    #[error("\
        Expected `{0}` to be a list attribute: `#[{k}({0}(..),)]`.",
        k = symbol::KLV_ATTR,
    )]
    ExpectedAsList(String),

    #[error("\
        Expected `{0}` to be a name-value attribute: `#[{k}({0} = <..>,)]`.",
        k = symbol::KLV_ATTR,
    )]
    ExpectedAsNameValue(String),

    #[error("\
        Expected `{0}` to be a path attribute: `#[{k}({0},)]`.",
        k = symbol::KLV_ATTR,
    )]
    ExpectedAsPath(String),

    // --------------------------------------------------
    // stream
    // --------------------------------------------------
    #[error("\
        Duplicate `{s}` attribute. Only one stream-type per struct is supported.",
        s = symbol::STREAM,
    )]
    DuplicateStream,

    // --------------------------------------------------
    // sentinel
    // --------------------------------------------------
    #[error("\
        Duplicate `{s}` attribute. Currently, only one sentinel is supported per struct.",
        s = symbol::SENTINEL,
    )]
    DuplicateSentinel,

    // --------------------------------------------------
    // break_on
    // --------------------------------------------------
    #[error("\
        Duplicate `{s}` attribute. Only one break condition per struct is supported.",
        s = symbol::BREAK_ON,
    )]
    DuplicateBreakOn,

    #[error("\
        `{s}` expects either a key literal (e.g. `{s} = 0xFF`) or a function path \
        (e.g. `{s} = my_break_fn`, where `fn(key, len) -> tinyklv::BreakType`).",
        s = symbol::BREAK_ON,
    )]
    InvalidBreakOn,

    // --------------------------------------------------
    // key / len
    // --------------------------------------------------
    #[error("\
        Missing required `{s}` field to describe how to encode/decode key values.",
        s = symbol::KEY,
    )]
    MissingKey,

    #[error("\
        Missing required `{s}` field to describe how to encode/decode length values.",
        s = symbol::LENGTH,
    )]
    MissingLength,

    #[error("\
        Duplicate `{s}` attribute. Only one method of encoding and decoding keys is allowed.",
        s = symbol::KEY,
    )]
    DuplicateKey,

    #[error("\
        Duplicate `{s}` attribute. Only one method of encoding and decoding lengths is allowed.",
        s = symbol::LENGTH,
    )]
    DuplicateLength,

    #[error("\
        Unknown field `{0}` in {k}/{l}.
Expected `{e} = <..>` or `{d} = <..>`.",
        k = symbol::KEY,
        l = symbol::LENGTH,
        e = symbol::ENCODER,
        d = symbol::DECODER,
    )]
    UnknownKeyLenField(String),

    #[error("\
        Duplicate `{e}` field. Only one method of encoding {k}/{l} is allowed.",
        e = symbol::ENCODER,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    DuplicateEncoderInKeyLen,

    #[error("\
        Duplicate `{d}` field. Only one method of decoding {k}/{l} is allowed.",
        d = symbol::DECODER,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    DuplicateDecoderInKeyLen,

    #[error("\
        Missing required `{e}` field for `{0}`.
If no encoding is required, use `#[{a}({aue})]`.",
        e = symbol::ENCODER,
        a = symbol::KLV_ATTR,
        aue = symbol::ALLOW_UNIMPLEMENTED_ENCODE,
    )]
    MissingEncInKeyLen(String),

    #[error("\
        Missing required `{d}` field for `{0}`.
If no decoding is required, use `#[{a}({aud})]`.",
        d = symbol::DECODER,
        a = symbol::KLV_ATTR,
        aud = symbol::ALLOW_UNIMPLEMENTED_DECODE,
    )]
    MissingDecInKeyLen(String),

    // --------------------------------------------------
    // default
    // --------------------------------------------------
    #[error("\
        Unknown default field: `{0}`.
Expected {s}.",
        s = symbol::CONT_DEFAULT_LIST_SYMBOLS,
    )]
    UnknownDefaultField(String),

    #[error("\
        Missing required `{s}` field for `{d}`.",
        s = symbol::TYPE,
        d = symbol::DEFAULT,
    )]
    MissingTypeInDefault,

    #[error("\
        Both `{e}` and `{d}` fields are missing: at least one is required.",
        e = symbol::ENCODER,
        d = symbol::DECODER,
    )]
    MissingEncDecInDefault,

    #[error("\
        Duplicate `{s}` field.
To add multiple default encoder/decoders for types, add another `{d}` attribute.",
        s = symbol::TYPE,
        d = symbol::DEFAULT,
    )]
    DuplicateTypeInDefault,

    #[error("\
        Duplicate `{s}` field. Only one default encoder is allowed for {0} type.
To add ways to encode said type, they must be done on a field-by-field basis and not using the `{d}` attribute.",
        s = symbol::ENCODER,
        d = symbol::DEFAULT,
    )]
    DuplicateEncoderInDefault(String),

    #[error("\
        Duplicate `{s}` field. Only one default decoder is allowed for {0} type.
To add ways to decode said type, they must be done on a field-by-field basis and not using the `{d}` attribute.",
        s = symbol::DECODER,
        d = symbol::DEFAULT,
    )]
    DuplicateDecoderInDefault(String),

    #[error("\
        Duplicate `{s}` field. Only required once, defaults to `false`.",
        s = symbol::VARIABLE_LENGTH,
    )]
    DuplicateVariableLengthInDefault,

    #[error("\
        Duplicate '{1}' found for `{d}({t} = {0})`. Only one default {1} is allowed per type.",
        d = symbol::DEFAULT,
        t = symbol::TYPE,
    )]
    DuplicateDefault(String, String),

    // --------------------------------------------------
    // field
    // --------------------------------------------------
    #[error("\
        Malformed `{s}` field. Expecting list: `{s}(..)`
Options include {fs}.",
        s = symbol::KLV_ATTR,
        fs = symbol::FIELD_SYMBOLS,
    )]
    MalformedField,

    #[error("\
        Unknown field: `{0}`.
Expected {s}.",
        s = symbol::FIELD_SYMBOLS
    )]
    UnknownFieldField(String),

    #[error("\
        Duplicate `{s}` field.
Currently only one key per field is supported.",
        s = symbol::KEY,
    )]
    DuplicateKeyInField,

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::ENCODER,
    )]
    DuplicateEncoderInField,

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::DECODER,
    )]
    DuplicateDecoderInField,

    #[error("\
        Duplicate `{s}` field. Only required once, will default to `false`.",
        s = symbol::VARIABLE_LENGTH,
    )]
    DuplicateVariableLengthInField,

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::LATEBIND,
    )]
    DuplicateLatebindInField,

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::DEFAULT_VALUE,
    )]
    DuplicateDefaultInField,

    #[error("\
        Missing required `{s}` field.",
        s = symbol::KEY,
    )]
    MissingKeyInField,

    #[error("\
No encoder is found for field `{0}: {1}`.

If encoding is not required, use `#[{k}({aue})]` on the struct.

Otherwise, you can:
    a. add a default encoder for all `{1}` types using `#[{k}({df}({t} = {1}, {e} = <..>)))]` on the struct
    b. add an encoder to `{0}` using `#[{k}({e} = <..>)]`
    c. set `#[{k}({fi})]` on the struct to fall back to try and use `tinyklv::EncodeValue` trait implementation",
        k = symbol::KLV_ATTR,
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        e = symbol::ENCODER,
        aue = symbol::ALLOW_UNIMPLEMENTED_ENCODE,
        fi = symbol::TRAIT_FALLBACK,
    )]
    UnimplementedEncode(String, String),

    #[error("\
No decoder is found for field `{0}: {1}`.

If decoding is not required, use `#[{k}({aud})]` on the struct.

Otherwise, you can:
    a. add a default decoder for all `{1}` types using `#[{k}({df}({t} = {1}, {d} = <..>)))]` on the struct
    b. add a decoder to `{0}` using `#[{k}({d} = <..>)]`
    c. set `#[{k}({fi})]` on the struct to fall back to try and use `tinyklv::DecodeValue` trait implementation",
        k = symbol::KLV_ATTR,
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        d = symbol::DECODER,
        aud = symbol::ALLOW_UNIMPLEMENTED_DECODE,
        fi = symbol::TRAIT_FALLBACK,
    )]
    UnimplementedDecode(String, String),

    #[error("\
Field `{0}: {1}` has `{v} = true` but no `{d} = ..` was set.
The trait fallback (`<{1} as ::tinyklv::DecodeValue<..>>::decode_value`) has no length argument - \
supply an explicit `{d}` or remove `{v}`.",
        v = symbol::VARIABLE_LENGTH,
        d = symbol::DECODER,
    )]
    VarlenFallbackRequiresExplicitDec(String, String),
}
/// [`Error`] implementation
impl Error {
    /// Converts this error to an owned [`std::borrow::Cow<str>`]
    ///
    /// Formats the error via its [`std::fmt::Display`] impl (provided by
    /// [`thiserror`]) and wraps it in [`std::borrow::Cow::Owned`]. The result
    /// is passed to [`crate::Ctxt::error_spanned_by`] so the diagnostic message
    /// is attached to the correct source span
    fn as_str(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
