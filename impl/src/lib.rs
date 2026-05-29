// --------------------------------------------------
// mods
// --------------------------------------------------
#[macro_use]
mod err;
mod ast;
mod ctxt;
mod expand;

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{DeriveInput, parse_macro_input};
use thiserror::Error;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
use crate::ast::symbol;
use crate::ctxt::Ctxt;

// --------------------------------------------------
// constants
// --------------------------------------------------
const CRATE_NAME: &str = "tinyklv";
const DERIVE_NAME: &str = "Klv";
const ATTR_NAME: &str = "klv";

#[proc_macro_derive(Klv, attributes(klv))]
/// [`tinyklv`](crate) proc-macro
pub fn klv_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(Debug, Error)]
/// [`tinyklv`](crate) proc-macro errors
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
    fn as_str(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
