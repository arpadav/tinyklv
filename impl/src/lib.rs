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
//! 3. Conditionally generates `EncodeValue`/`EncodeFrame` and
//!    `DecodeValue`/`DecodePartial`/`ResumePartial`/`SeekSentinel` implementations
//!    depending on the container's framing and on which fields carry encoders
//!    and decoders (`expand` module). `DecodeFrame` is not generated: `tinyklv`'s
//!    blanket impl provides it for every `SeekSentinel` + `DecodeValue` type
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
use crate::{ast::symbol, ctxt::Ctxt};

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
/// present. A missing encoder, on a field, on the framing attribute (the
/// `key(..)` + `len(..)` pair or the fused `keylen(..)` codec) or on the
/// `unknown(..)` codec of a struct whose `#[klv(unknown)]` field keeps its
/// frames, is a compile error unless the container sets
/// `allow_unimplemented_encode`, in which case no encode impl is generated.
/// Missing decoders, `unknown(..)`'s included, behave the same under
/// `allow_unimplemented_decode`, suppressing the decode impls
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

    #[error(
        "\
        `{0}` takes a path, not a literal: `{0} = my::path`, or the path in a string, \
`{0} = \"my::path\"`."
    )]
    LiteralSetting(String),

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

    #[error("\
        `{s}` is searched for in the input's bytes, which a `&str` `{st}` does not give; use \
`{st} = &[u8]`, the default, or remove `{s}`.",
        s = symbol::SENTINEL,
        st = symbol::STREAM,
    )]
    SentinelOnStrStream,

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
        (e.g. `{s} = my_break_fn`, where `fn(&key, &mut input) -> tinyklv::BreakType`).",
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
        Missing framing attributes to describe how to encode/decode keys and lengths.
Use `{k}(..)` + `{l}(..)`, or the fused `{kl}(..)` codec.",
        k = symbol::KEY,
        l = symbol::LENGTH,
        kl = symbol::KEYLEN,
    )]
    MissingFraming,

    #[error("\
        Conflicting framing attributes. The fused `{kl}(..)` codec is mutually exclusive with \
separate `{k}(..)`/`{l}(..)`.
Use either `{kl}` OR `{k}`+`{l}`, not both.",
        kl = symbol::KEYLEN,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    KeylenWithKeyOrLen,

    #[error("\
        `{s}` is not supported with `{kl}`. Remove `{s}`, or use `{k}(..)` + `{l}(..)`.",
        s = symbol::SENTINEL,
        kl = symbol::KEYLEN,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    KeylenWithSentinel,

    #[error("\
        `{s}({i})` is not supported with `{kl}`: the `{kl}` codec decides each frame's framing; \
remove `{i}`, or use `{k}(..)` + `{l}(..)` in place of `{kl}(..)`.",
        s = symbol::SIZE,
        i = symbol::IMPLICIT,
        kl = symbol::KEYLEN,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    ImplicitInKeylen,

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
        Duplicate `{kl}` attribute. Only one fused key/length codec per struct is allowed.",
        kl = symbol::KEYLEN,
    )]
    DuplicateKeylen,

    #[error("\
        Unknown field `{0}` in {k}/{l}.
Expected `{e} = <..>`, `{d} = <..>`, or `{s}(..)`.",
        k = symbol::KEY,
        l = symbol::LENGTH,
        e = symbol::ENCODER,
        d = symbol::DECODER,
        s = symbol::SIZE,
    )]
    UnknownHeaderField(String),

    #[error("\
        Unknown field `{0}` in {kl}.
Expected `{e} = <..>` or `{d} = <..>`.",
        kl = symbol::KEYLEN,
        e = symbol::ENCODER,
        d = symbol::DECODER,
    )]
    UnknownKeylenField(String),

    #[error("\
        Duplicate `{e}` field. Only one method of encoding {k}/{l}/{kl} is allowed.",
        e = symbol::ENCODER,
        k = symbol::KEY,
        l = symbol::LENGTH,
        kl = symbol::KEYLEN,
    )]
    DuplicateEncoderInHeader,

    #[error("\
        An `{e}` in {k}/{l}/{kl} takes no `&` or `*` sigil.
The derive passes the key and length by value; sigils apply to field encoders only.",
        e = symbol::ENCODER,
        k = symbol::KEY,
        l = symbol::LENGTH,
        kl = symbol::KEYLEN,
    )]
    SigilOnHeaderEncoder,

    #[error("\
        Duplicate `{d}` field. Only one method of decoding {k}/{l}/{kl} is allowed.",
        d = symbol::DECODER,
        k = symbol::KEY,
        l = symbol::LENGTH,
        kl = symbol::KEYLEN,
    )]
    DuplicateDecoderInHeader,

    #[error("\
        Duplicate `{s}` field. Only one {s} per {k}/{l} is allowed.",
        s = symbol::SIZE,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    DuplicateSizeInHeader,

    #[error("\
        A `{l}(..)` with `{s}({e} = 0)` has no bytes to write a value's length in, so every \
value with a length prefix is lost; use `{e} = 1` or more, or remove `{e}`.",
        l = symbol::LENGTH,
        s = symbol::SIZE,
        e = symbol::EXACT,
    )]
    LenZeroWidth,

    #[error("\
        Unknown `{s}` sub-attribute in {k}/{l}: `{0}`.
Expected `{e} = <..>` or `{r} = <..>`; `{lt}` goes in a field's `{s}(..)`.",
        s = symbol::SIZE,
        k = symbol::KEY,
        l = symbol::LENGTH,
        e = symbol::EXACT,
        r = symbol::RESERVE,
        lt = symbol::LENGTHED,
    )]
    UnknownSizeFieldInHeader(String),

    #[error("\
        `{s}(..)` is not supported in `{kl}(..)`: the `{kl}` codec decides each frame's framing; \
put `{s}(..)` on a field, or on `{k}(..)`/`{l}(..)` in place of `{kl}(..)`.",
        s = symbol::SIZE,
        kl = symbol::KEYLEN,
        k = symbol::KEY,
        l = symbol::LENGTH,
    )]
    SizeInKeylen,

    #[error("\
        Missing required `{e}` field for `{0}`.
If no encoding is required, use `#[{a}({aue})]`.",
        e = symbol::ENCODER,
        a = symbol::KLV_ATTR,
        aue = symbol::ALLOW_UNIMPLEMENTED_ENCODE,
    )]
    MissingEncInCodec(String),

    #[error("\
        Missing required `{d}` field for `{0}`.
If no decoding is required, use `#[{a}({aud})]`.",
        d = symbol::DECODER,
        a = symbol::KLV_ATTR,
        aud = symbol::ALLOW_UNIMPLEMENTED_DECODE,
    )]
    MissingDecInCodec(String),

    // --------------------------------------------------
    // unknown
    // --------------------------------------------------
    #[error("\
        Duplicate `{u}` attribute. Only one codec for unknown frames per struct is allowed.",
        u = symbol::UNKNOWN_FRAMES,
    )]
    DuplicateUnknown,

    #[error("\
        Unknown field `{0}` in `{u}(..)`.
Expected `{e} = <..>` or `{d} = <..>`.",
        u = symbol::UNKNOWN_FRAMES,
        e = symbol::ENCODER,
        d = symbol::DECODER,
    )]
    UnknownFieldInUnknown(String),

    #[error("\
        Duplicate `{e}` field in `{u}(..)`. Only one method of encoding unknown frames is \
allowed.",
        e = symbol::ENCODER,
        u = symbol::UNKNOWN_FRAMES,
    )]
    DuplicateEncoderInUnknown,

    #[error("\
        Duplicate `{d}` field in `{u}(..)`. Only one method of decoding unknown frames is \
allowed.",
        d = symbol::DECODER,
        u = symbol::UNKNOWN_FRAMES,
    )]
    DuplicateDecoderInUnknown,

    #[error("\
        `{dk}` rejects a frame whose key no field declares, and `{u}(..)` decodes it; remove \
one of them.",
        dk = symbol::DENY_UNKNOWN_KEYS,
        u = symbol::UNKNOWN_FRAMES,
    )]
    UnknownWithDenyUnknownKeys,

    #[error("\
        `{u}` is set twice on this field; remove one.",
        u = symbol::UNKNOWN_FRAMES,
    )]
    DuplicateUnknownInField,

    #[error("\
        Only one field per struct may be `#[{k}({u})]`: it keeps every frame whose key no field \
declares. Remove `{u}` from this field or from the earlier one.",
        k = symbol::KLV_ATTR,
        u = symbol::UNKNOWN_FRAMES,
    )]
    MultipleUnknownFields,

    #[error("\
        An `{u}` field takes no other setting: the struct's `{u}({d} = <..>, {e} = <..>)` decodes \
and encodes its frames. Remove the other settings from this field.",
        u = symbol::UNKNOWN_FRAMES,
        d = symbol::DECODER,
        e = symbol::ENCODER,
    )]
    UnknownFieldWithSetting,

    #[error("\
        An `{u}` field keeps the frames the struct's `{u}({d} = <..>, {e} = <..>)` decodes, and \
this struct has no `{u}(..)`. Add one to the struct's `#[{k}(..)]`, or remove `{u}` from this \
field.",
        u = symbol::UNKNOWN_FRAMES,
        d = symbol::DECODER,
        e = symbol::ENCODER,
        k = symbol::KLV_ATTR,
    )]
    UnknownFieldWithoutCodec,

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
        No `{df}({t} = {0}, ..)` sets an `{e}` or a `{d}`: at least one is required.",
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        e = symbol::ENCODER,
        d = symbol::DECODER,
    )]
    MissingEncDecInDefault(String),

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
        Duplicate `{s}` field. Only one {s} per default is allowed.",
        s = symbol::SIZE,
    )]
    DuplicateSizeInDefault,

    #[error("\
        `{l}` belongs to the decoder, and this `{df}(..)` has no `{d}`; add a `{d}` here, or \
remove `{l}`.",
        l = symbol::LENGTHED,
        d = symbol::DECODER,
        df = symbol::DEFAULT,
    )]
    SizeLengthedWithoutDecInDefault,

    #[error("\
        Duplicate {1} for `{d}({t} = {0})`. Only one is allowed per type; remove one.",
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
        A `{k} = {o}(..)` field takes no `{e}`, `{d}` or `{s}(..)`: each arm's value uses the \
codec of its type, from the struct's `{df}({t} = <..>, ..)` or `{fi}`. Remove this setting from \
the field.",
        k = symbol::KEY,
        o = symbol::ONEOF,
        e = symbol::ENCODER,
        d = symbol::DECODER,
        s = symbol::SIZE,
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        fi = symbol::TRAIT_FALLBACK,
    )]
    OneofFieldWithCodec,

    #[error("\
        A `{k} = {o}(..)` names at least one arm, `K => Enum::Variant(Type)`; add one, or give \
the field one key with `{k} = K`.",
        k = symbol::KEY,
        o = symbol::ONEOF,
    )]
    OneofWithoutArms,

    #[error(
        "\
        The arm of key `{0}` already wraps its value in this variant, and encode writes a variant \
under one key; give each arm its own variant, or remove one."
    )]
    OneofDuplicateVariant(String),

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
        Duplicate `{s}` field. Only one {s} per field is allowed.",
        s = symbol::SIZE,
    )]
    DuplicateSizeInField,

    #[error("\
        Unknown `{s}` sub-attribute: `{0}`.
Expected {sf}.",
        s = symbol::SIZE,
        sf = symbol::SIZE_FIELDS,
    )]
    UnknownSizeField(String),

    #[error("\
        Conflicting `{s}` byte count. `{e}` and `{r}` are mutually exclusive - a `{s}` is either an \
exact width or a soft capacity estimate, not both.",
        s = symbol::SIZE,
        e = symbol::EXACT,
        r = symbol::RESERVE,
    )]
    SizeExactReserveConflict,

    #[error(
        "\
        `{0} = {1}` is more bytes than a `Vec<u8>` can hold (`isize::MAX`), and the encode \
reserves them, which panics; use a smaller `{0}`."
    )]
    SizeTooLarge(String, String),

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::LATEBIND,
    )]
    DuplicateLatebindInField,

    #[error("\
        Duplicate `{s}` field.",
        s = symbol::BREAK_ON,
    )]
    DuplicateBreakOnInField,

    #[error("\
        Only one of `{d}` and `{s} = <fn>` is allowed, once; remove the other.",
        d = symbol::SKIP_IF_DEFAULT,
        s = symbol::SKIP_IF,
    )]
    SkipIfConflict,

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

    #[error(
        "\
        Key `{0}` is already a key of field `{1}`, which the frames of `{0}` decode into, so \
this one never decodes; give one of them another key."
    )]
    SharedKey(String, String),

    #[error("\
No encoder is found for field `{0}: {1}`.

If encoding is not required, use `#[{k}({aue})]` on the struct.

Otherwise, you can:
    a. add a default encoder for all `{1}` types using `#[{k}({df}({t} = {1}, {e} = <..>))]` on the struct
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
    a. add a default decoder for all `{1}` types using `#[{k}({df}({t} = {1}, {d} = <..>))]` on the struct
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
No encoder is found for the `{2}({1})` arm of field `{0}`.

If encoding is not required, use `#[{k}({aue})]` on the struct.

Otherwise, you can:
    a. add a default encoder for all `{1}` types using `#[{k}({df}({t} = {1}, {e} = <..>))]` on the struct
    b. set `#[{k}({fi})]` on the struct to fall back to try and use `tinyklv::EncodeValue` trait implementation",
        k = symbol::KLV_ATTR,
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        e = symbol::ENCODER,
        aue = symbol::ALLOW_UNIMPLEMENTED_ENCODE,
        fi = symbol::TRAIT_FALLBACK,
    )]
    UnimplementedArmEncode(String, String, String),

    #[error("\
No decoder is found for the `{2}({1})` arm of field `{0}`.

If decoding is not required, use `#[{k}({aud})]` on the struct.

Otherwise, you can:
    a. add a default decoder for all `{1}` types using `#[{k}({df}({t} = {1}, {d} = <..>))]` on the struct
    b. set `#[{k}({fi})]` on the struct to fall back to try and use `tinyklv::DecodeValue` trait implementation",
        k = symbol::KLV_ATTR,
        df = symbol::DEFAULT,
        t = symbol::TYPE,
        d = symbol::DECODER,
        aud = symbol::ALLOW_UNIMPLEMENTED_DECODE,
        fi = symbol::TRAIT_FALLBACK,
    )]
    UnimplementedArmDecode(String, String, String),

    #[error("\
Field `{0}: {1}` has `{s}({l})` but no `{d} = ..` was set.
The trait fallback (`<{1} as ::tinyklv::DecodeValue<..>>::decode_value`) has no length argument - \
supply an explicit `{d}` or change the `{s}`.",
        s = symbol::SIZE,
        l = symbol::LENGTHED,
        d = symbol::DECODER,
    )]
    SizeLengthedFallbackRequiresExplicitDec(String, String),

    #[error("\
        `{l}` belongs to the decoder; give this field its own `{d}`, or put `{s}({l})` on the \
`{df}`.",
        l = symbol::LENGTHED,
        d = symbol::DECODER,
        s = symbol::SIZE,
        df = symbol::DEFAULT,
    )]
    SizeLengthedOnDefaultDec,

    #[error("\
        A `{l}` decoder is passed the value's length, and an `{i}` value has none; add \
`{e} = N` to pass it `N`, or remove `{l}`.",
        l = symbol::LENGTHED,
        i = symbol::IMPLICIT,
        e = symbol::EXACT,
    )]
    SizeImplicitLengthedWithoutExact,

    #[error("\
        The value of field `{field}` is {value_bytes} bytes but `{l}({s}({e} = {width}))` can \
encode lengths only up to {max_len}.",
        l = symbol::LENGTH,
        s = symbol::SIZE,
        e = symbol::EXACT,
    )]
    LenPrefixTooNarrow {
        field: String,
        value_bytes: usize,
        width: usize,
        max_len: usize,
    },
}
/// [`Error`] implementation
impl Error {
    /// Converts this error to an owned [`std::borrow::Cow<str>`]
    ///
    /// Formats the error via its [`std::fmt::Display`] impl (provided by
    /// [`thiserror`]) and wraps it in [`std::borrow::Cow::Owned`]. The result
    /// is passed to [`crate::Ctxt::error_spanned_by`] so the diagnostic message
    /// is attached to the correct source span
    fn as_str(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
