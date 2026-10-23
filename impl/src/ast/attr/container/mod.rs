//! Parsing of container-level `#[klv(..)]` attributes
//!
//! Defines [`Container`] (the raw parsed form of every recognised attribute on
//! a `#[derive(Klv)]` struct) and [`ContainerParsed`] (the validated form that
//! resolves exactly one framing mode, [`ContainerMode`]). Sub-modules handle the
//! individual attribute parsers: `break_on`, `codec` (the state of every codec attribute),
//! `default`, `header` (the `key` and `len` lists, and the three frame header states), `keylen`
//! (the fused `keylen` list), `sentinel` (the bytes each packet starts with, and its own packet
//! length codec), `stream` and `unknown` (the codec of the frames whose key no field declares)
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod break_on;
mod codec;
mod default;
mod header;
mod keylen;
mod sentinel;
mod stream;
mod unknown;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use break_on::BreakOnSpec;
pub(super) use codec::CodecAttr;
pub(super) use default::DefaultXcoder;
pub(super) use unknown::UnknownXcoder;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{Ctxt, ast::types::ungrouped, symbol};
use break_on::BreakOn;
use codec::{CodecFields, CodecSlot};
use default::PartialDefault;
use header::{HeaderAttrs, HeaderXcoder};
use keylen::KeylenXcoder;
use sentinel::{ParsedSentinel, Sentinel};
use stream::Stream;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;
use syn::{Token, punctuated::Punctuated};

/// The validated key/length framing mode of a container
///
/// Produced by [`ContainerParsed::from_cont`] from the mutually-exclusive
/// `key`+`len` and `keylen` attributes. Modelling the two as one enum makes the
/// illegal "both present" and "neither present" states unrepresentable past
/// validation, and so is a fused `sentinel` without a packet length xcoder. The `expand` pass
/// picks the decode loop's frame header and body, the frame header written around each encoded
/// value, and the packet framing ([`Self::packet`]), from it
#[derive(Debug, Clone)]
pub(crate) enum ContainerMode {
    /// Key + len: separate `key`/`len` xcoders; a known key's value with a length prefix is sliced
    /// via `take(len)`, while an `unknown(..)` decoder reads the main stream after the key
    Separate {
        /// The key xcoder
        key: Box<HeaderXcoder>,
        /// The length xcoder
        len: Box<HeaderXcoder>,
        /// The sentinel, if any, and its own packet length xcoder, which frames the packet in
        /// place of `len`
        sentinel: Option<Sentinel<Option<Box<HeaderXcoder>>>>,
    },
    /// Fused mode: one `keylen` codec, whose decoder returns `(key, Framing)` so framing can
    /// depend on the key, and whose encoder writes the header from `(key, value_len)`
    Fused {
        /// The fused key+length codec
        keylen: Box<KeylenXcoder>,
        /// The sentinel, if any, and its packet length xcoder: the `keylen` codec has no
        /// standalone length to frame the packet with
        sentinel: Option<Sentinel<Box<HeaderXcoder>>>,
    },
}
/// [`ContainerMode`] implementation
impl ContainerMode {
    /// The framing of the container's packets
    ///
    /// # Returns
    ///
    /// The bytes each packet starts with, and the xcoder of each packet's length: the sentinel's
    /// own or, beside `key(..)` and `len(..)`, the container's `len(..)`; or `None` without a
    /// sentinel
    pub(crate) fn packet(&self) -> Option<(&syn::Lit, &HeaderXcoder)> {
        match self {
            Self::Separate { len, sentinel, .. } => {
                sentinel.as_ref().map(|Sentinel { value, len: own }| {
                    // without its own, the container's `len(..)` frames the packet
                    (value, own.as_deref().unwrap_or(len))
                })
            }
            Self::Fused { sentinel, .. } => sentinel
                .as_ref()
                .map(|Sentinel { value, len }| (value, &**len)),
        }
    }

    /// Whether the container's `len(..)` frames each packet
    ///
    /// # Returns
    ///
    /// `true` for a key + len sentinel without its own `len(..)`
    pub(crate) fn len_frames_packet(&self) -> bool {
        matches!(
            self,
            Self::Separate {
                sentinel: Some(Sentinel { len: None, .. }),
                ..
            }
        )
    }

    /// Every `len(..)` the container declares
    ///
    /// # Returns
    ///
    /// The key + len one, then the sentinel's own
    pub(crate) fn lens(&self) -> impl Iterator<Item = &HeaderXcoder> {
        let (len, own) = match self {
            Self::Separate { len, sentinel, .. } => (
                Some(len),
                sentinel.as_ref().and_then(|sentinel| sentinel.len.as_ref()),
            ),
            Self::Fused { sentinel, .. } => (None, sentinel.as_ref().map(|sentinel| &sentinel.len)),
        };
        len.into_iter().chain(own).map(Box::as_ref)
    }
}

/// The missing codecs a container allows: under `allow_unimplemented_encode` /
/// `allow_unimplemented_decode` a missing encoder / decoder is no error, and its impl is not
/// generated
#[derive(Debug, Clone, Copy)]
pub(super) struct AllowUnimplemented {
    /// Whether a missing encoder is allowed
    pub encode: bool,

    /// Whether a missing decoder is allowed
    pub decode: bool,
}

#[derive(Debug)]
/// Raw parsed form of every recognised container-level `#[klv(..)]` attribute
///
/// The attribute parser accumulates whatever it finds, so every attribute may be
/// absent here. Which framing attributes must be present, and which may not
/// appear together, is validated by [`ContainerParsed::from_cont`], which
/// reports the violations to the [`Ctxt`]
pub(crate) struct Container {
    /// The stream type used for decoding, e.g. `&[u8]` (defaults when absent)
    pub stream: Option<syn::Type>,

    /// The sentinel each packet starts with, e.g. `b"HEARTBEAT"`, and its own packet length
    /// xcoder, if any
    pub sentinel: Option<ParsedSentinel>,

    /// The break condition: a key literal (stop on match) or a `fn(&key, &mut input) -> BreakType`
    /// path
    pub break_on: Option<BreakOnSpec>,

    /// The key and length xcoder pairs and the fused key+length codec (mutually exclusive with
    /// `key`/`len`): each absent, present but unusable, or parsed
    headers: HeaderAttrs,

    /// The codec of the frames whose key no field declares: absent, present but unusable, or
    /// parsed
    unknown: CodecAttr<UnknownXcoder>,

    /// Per-type default xcoders keyed by the Rust type they apply to, its invisible groups taken
    /// away ([`ungrouped`]), as a field's type is looked up
    pub defaults: HashMap<syn::Type, DefaultXcoder>,

    /// Whether debug logging is enabled for this container
    pub debug: Option<syn::Path>,

    /// Whether unknown keys cause a decode error rather than being silently skipped
    pub deny_unknown_keys: Option<syn::Path>,

    /// Whether a header-sized value its decoder fails on, or leaves bytes of unread, is an error
    pub strict: Option<syn::Path>,

    /// Whether fields with no decoder are allowed (suppresses missing-decoder errors)
    pub allow_unimplemented_decode: Option<syn::Path>,

    /// Whether fields with no encoder are allowed (suppresses missing-encoder errors)
    pub allow_unimplemented_encode: Option<syn::Path>,

    /// Whether to fall back to [`tinyklv::EncodeValue`]/[`tinyklv::DecodeValue`] trait
    /// impls for fields with no explicit xcoder
    pub trait_fallback: Option<syn::Path>,
}
/// [`Container`] implementation
impl Container {
    /// Parses the `#[klv(..)]` attributes from a container derive input
    ///
    /// Iterates over all attributes on `item`, filters to those matching the
    /// top-level `klv` attribute name, and dispatches each nested meta entry
    /// to the appropriate sub-parser. Validation errors (unknown attributes,
    /// duplicate entries, wrong meta form) are pushed to `cx` rather than
    /// returned, so that as many diagnostics as possible are gathered in one
    /// pass. Only struct containers are supported; enum support is not implemented
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context used to record diagnostics
    /// * `item` - The full derive input whose attributes are to be parsed
    ///
    /// # Returns
    ///
    /// A [`Container`] populated with whatever valid attributes were found; any
    /// errors are deferred to [`cx`] for the caller to check via [`Ctxt::check`]
    pub fn from_ast(cx: &Ctxt, item: &syn::DeriveInput) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut stream = None;
        let mut sentinel = None;
        let mut break_on = None;
        let mut headers = HeaderAttrs::default();
        let mut unknown = CodecAttr::default();
        let mut defaults: Vec<(syn::Type, DefaultXcoder)> = Vec::new();
        let mut debug = None;
        let mut deny_unknown_keys = None;
        let mut strict = None;
        let mut allow_unimplemented_decode = None;
        let mut allow_unimplemented_encode = None;
        let mut trait_fallback = None;
        // --------------------------------------------------
        // loop through attrs
        // --------------------------------------------------
        for attr in &item.attrs {
            // --------------------------------------------------
            // only look for klv attr(s)
            // --------------------------------------------------
            if attr.path() != symbol::KLV_ATTR {
                continue;
            }
            // --------------------------------------------------
            // split using comma
            // --------------------------------------------------
            let nested =
                match attr.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated) {
                    Ok(nested) => nested,
                    Err(err) => {
                        cx.syn_error(err);
                        continue;
                    }
                };
            // --------------------------------------------------
            // loop through comma blocks
            // --------------------------------------------------
            for meta in nested {
                let name = symbol::Symbol::from(meta.path());
                // --------------------------------------------------
                // the codec attributes, `key`, `len`, `keylen` and `unknown`, in any form, each
                // recorded on the state of its slot
                // --------------------------------------------------
                if let Some(slot) = CodecSlot::from_symbol(name) {
                    match slot {
                        CodecSlot::Key => headers.key.on_meta(cx, &meta, slot),
                        CodecSlot::Length => headers.len.on_meta(cx, &meta, slot),
                        CodecSlot::Keylen => headers.keylen.on_meta(cx, &meta, slot),
                        CodecSlot::Unknown => unknown.on_meta(cx, &meta, slot),
                    }
                    continue;
                }
                // --------------------------------------------------
                // `sentinel`, a name-value or a list
                // --------------------------------------------------
                if name == symbol::SENTINEL {
                    match sentinel {
                        Some(_) => cx.error_spanned_by(&meta, err!(DuplicateSentinel)),
                        None => {
                            sentinel = ParsedSentinel::try_from(&meta)
                                .map_err(|err| cx.syn_error(err))
                                .ok();
                        }
                    }
                    continue;
                }
                match meta {
                    // --------------------------------------------------
                    // handle all: `syn::MetaList`
                    // e.g. `key(enc = <..>, dec = <..>),`
                    // --------------------------------------------------
                    syn::Meta::List(list) => match name {
                        // --------------------------------------------------
                        // an entry keeps what parsed before an error, and merges into the
                        // earlier entries of its type
                        // --------------------------------------------------
                        symbol::DEFAULT => {
                            let new = match DefaultXcoder::try_from(&list) {
                                Ok(new) => new,
                                Err(PartialDefault { entry, error }) => {
                                    cx.syn_error(error);
                                    DefaultXcoder {
                                        reported: true,
                                        ..entry
                                    }
                                }
                            };
                            // `None` when the entry has no `typ`, or its parse stopped before one;
                            // either error is reported above
                            let Some(typ) = new.typ.as_ref().map(ungrouped) else {
                                continue;
                            };
                            match defaults.iter_mut().find(|(old, _)| *old == typ) {
                                Some((_, old)) => old.merge(cx, new),
                                None => defaults.push((typ, new)),
                            }
                        }
                        // --------------------------------------------------
                        // non lists
                        // --------------------------------------------------
                        symbol::STREAM => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsNameValue(symbol::STREAM)),
                        ),
                        symbol::BREAK_ON => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsNameValue(symbol::BREAK_ON)),
                        ),
                        symbol::DEBUG => {
                            cx.error_spanned_by(&list.path, err!(ExpectedAsPath(symbol::DEBUG)));
                        }
                        symbol::DENY_UNKNOWN_KEYS => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsPath(symbol::DENY_UNKNOWN_KEYS)),
                        ),
                        symbol::STRICT => {
                            cx.error_spanned_by(&list.path, err!(ExpectedAsPath(symbol::STRICT)));
                        }
                        symbol::ALLOW_UNIMPLEMENTED_DECODE => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsPath(symbol::ALLOW_UNIMPLEMENTED_DECODE)),
                        ),
                        symbol::ALLOW_UNIMPLEMENTED_ENCODE => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsPath(symbol::ALLOW_UNIMPLEMENTED_ENCODE)),
                        ),
                        symbol::TRAIT_FALLBACK => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsPath(symbol::TRAIT_FALLBACK)),
                        ),
                        _ => cx.error_spanned_by(
                            &list.path,
                            err!(UnknownContainerListAttribute(list.path)),
                        ),
                    },
                    // --------------------------------------------------
                    // handle all: `syn::MetaNameValue`
                    // e.g. `sentinel = <value>,`
                    // --------------------------------------------------
                    syn::Meta::NameValue(nv) => match name {
                        symbol::STREAM => match stream {
                            Some(_) => cx.error_spanned_by(&nv, err!(DuplicateStream)),
                            None => {
                                stream =
                                    Stream::try_from(&nv).map_err(|err| cx.syn_error(err)).ok();
                            }
                        },
                        symbol::BREAK_ON => match break_on {
                            Some(_) => cx.error_spanned_by(&nv, err!(DuplicateBreakOn)),
                            None => {
                                break_on =
                                    BreakOn::try_from(&nv).map_err(|err| cx.syn_error(err)).ok();
                            }
                        },
                        // --------------------------------------------------
                        // non name-values
                        // --------------------------------------------------
                        symbol::DEFAULT => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::DEFAULT)));
                        }
                        symbol::DEBUG => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsPath(symbol::DEBUG)));
                        }
                        symbol::DENY_UNKNOWN_KEYS => cx.error_spanned_by(
                            &nv.path,
                            err!(ExpectedAsPath(symbol::DENY_UNKNOWN_KEYS)),
                        ),
                        symbol::STRICT => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsPath(symbol::STRICT)));
                        }
                        symbol::ALLOW_UNIMPLEMENTED_DECODE => cx.error_spanned_by(
                            &nv.path,
                            err!(ExpectedAsPath(symbol::ALLOW_UNIMPLEMENTED_DECODE)),
                        ),
                        symbol::ALLOW_UNIMPLEMENTED_ENCODE => cx.error_spanned_by(
                            &nv.path,
                            err!(ExpectedAsPath(symbol::ALLOW_UNIMPLEMENTED_ENCODE)),
                        ),
                        symbol::TRAIT_FALLBACK => cx.error_spanned_by(
                            &nv.path,
                            err!(ExpectedAsPath(symbol::TRAIT_FALLBACK)),
                        ),
                        _ => cx.error_spanned_by(
                            &nv.path,
                            err!(UnknownContainerNameValueAttribute(nv.path)),
                        ),
                    },
                    // --------------------------------------------------
                    // handle all: `syn::Path` (basically idents/flags)
                    // e.g. `allow_unimplemented_decode,`
                    // --------------------------------------------------
                    syn::Meta::Path(path) => {
                        let flag = match name {
                            symbol::DEBUG => &mut debug,
                            symbol::DENY_UNKNOWN_KEYS => &mut deny_unknown_keys,
                            symbol::STRICT => &mut strict,
                            symbol::ALLOW_UNIMPLEMENTED_DECODE => &mut allow_unimplemented_decode,
                            symbol::ALLOW_UNIMPLEMENTED_ENCODE => &mut allow_unimplemented_encode,
                            symbol::TRAIT_FALLBACK => &mut trait_fallback,
                            // --------------------------------------------------
                            // non paths
                            // --------------------------------------------------
                            symbol::DEFAULT => {
                                cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::DEFAULT)));
                                continue;
                            }
                            symbol::STREAM => {
                                cx.error_spanned_by(
                                    &path,
                                    err!(ExpectedAsNameValue(symbol::STREAM)),
                                );
                                continue;
                            }
                            symbol::BREAK_ON => {
                                cx.error_spanned_by(
                                    &path,
                                    err!(ExpectedAsNameValue(symbol::BREAK_ON)),
                                );
                                continue;
                            }
                            _ => {
                                cx.error_spanned_by(&path, err!(UnknownContainerAttribute(path)));
                                continue;
                            }
                        };
                        // --------------------------------------------------
                        // a flag is set once
                        // --------------------------------------------------
                        match flag {
                            Some(_) => cx.error_spanned_by(&path, err!(DuplicateFlag(path))),
                            None => *flag = Some(path),
                        }
                    }
                }
            }
        }
        // --------------------------------------------------
        // unimplemented encode / decode errors, spanned on the `key`/`len`/`keylen` path, or the
        // sentinel's own `len`, whose `enc`/`dec` is missing; `unknown(..)`'s are reported once
        // the fields are read, since its `enc` is needed only to re-emit the frames the
        // `#[klv(unknown)]` field keeps
        // --------------------------------------------------
        let allow = AllowUnimplemented {
            encode: allow_unimplemented_encode.is_some(),
            decode: allow_unimplemented_decode.is_some(),
        };
        headers.report_missing(cx, allow);
        if let Some(Sentinel {
            len: Some((path, len)),
            ..
        }) = &sentinel
        {
            len.report_missing(cx, path, allow);
        }
        // --------------------------------------------------
        // each type's merged `default(..)` entries are checked together, in the order the types
        // first appear
        // --------------------------------------------------
        for (typ, default) in &mut defaults {
            default.check(cx, typ);
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Container {
            stream: stream.and_then(|x| x.0),
            sentinel,
            break_on: break_on.and_then(|x| x.0),
            headers,
            unknown,
            defaults: defaults.into_iter().collect(),
            debug,
            deny_unknown_keys,
            strict,
            allow_unimplemented_decode,
            allow_unimplemented_encode,
            trait_fallback,
        }
    }

    /// The missing codecs the container allows
    ///
    /// # Returns
    ///
    /// Whether it sets `allow_unimplemented_encode` and `allow_unimplemented_decode`
    pub(super) fn allow_unimplemented(&self) -> AllowUnimplemented {
        AllowUnimplemented {
            encode: self.allow_unimplemented_encode.is_some(),
            decode: self.allow_unimplemented_decode.is_some(),
        }
    }
}

/// Validated container attributes where required fields are guaranteed present
///
/// Produced from a [`Container`] by [`ContainerParsed::from_cont`], which
/// resolves the key/length framing into a [`ContainerMode`] - verifying that
/// exactly one of `key`+`len` or `keylen` is present and emitting compile
/// errors via [`Ctxt`] otherwise
pub(crate) struct ContainerParsed {
    /// The stream type used for decoding; `None` means `&[u8]` will be used
    pub stream: Option<syn::Type>,

    /// The validated key/length framing mode (key + len or keylen), with its `sentinel`
    pub mode: ContainerMode,

    /// The break condition, if any, checked right after each frame's key in either mode
    pub break_on: Option<BreakOnSpec>,

    /// Whether debug logging is enabled for this container
    pub debug: Option<syn::Path>,

    /// Whether unknown keys cause a decode error rather than being silently skipped
    pub deny_unknown_keys: Option<syn::Path>,

    /// Whether a header-sized value its decoder fails on, or leaves bytes of unread, is an error
    pub strict: Option<syn::Path>,
}
/// [`ContainerParsed`] implementation
impl ContainerParsed {
    /// Converts a raw [`Container`] into a validated [`ContainerParsed`]
    ///
    /// Resolves the key/length framing into a [`ContainerMode`]: exactly one of
    /// the separate `key`+`len` pair or the fused `keylen` codec must be
    /// present. Records errors in `cx` for the conflicting, half-specified, or
    /// absent cases and for a `sentinel` without its own `len(..)` on a fused container, and
    /// returns `None` for them. It also records an error for `deny_unknown_keys` beside
    /// `unknown(..)` and a `len(size(exact = 0))`, the sentinel's own included, and
    /// still returns the container. The `sentinel`/`unknown`/`len`
    /// checks run before any early return, so they surface in the same compile as a framing
    /// conflict. An attribute that is present but unusable (it failed to parse,
    /// or was not written as a list) counts as present: its own error is not
    /// joined by a missing-`key`/`len`/framing one, and a `keylen` beside any
    /// `key`/`len` still reports the conflict. The `unknown(..)` state is returned as parsed, to
    /// be paired with the field that keeps its frames
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording missing-field diagnostics
    /// * `name` - The struct ident, used as the span anchor for error messages
    /// * `cont` - The raw parsed container attributes to validate
    ///
    /// # Returns
    ///
    /// The [`ContainerParsed`] and the `unknown(..)` state when exactly one framing mode is fully
    /// specified, and a fused one's `sentinel` has its own `len(..)`, or `None` otherwise (errors
    /// are recorded in `cx`)
    pub(super) fn from_cont(
        cx: &Ctxt,
        name: &syn::Ident,
        cont: Container,
    ) -> Option<(Self, CodecAttr<UnknownXcoder>)> {
        // --------------------------------------------------
        // a `sentinel` without its own packet length beside any `keylen(..)`, reported before the
        // framing resolution can return early
        // --------------------------------------------------
        if cont.headers.keylen.is_present()
            && let Some(Sentinel { value, len: None }) = &cont.sentinel
        {
            cx.error_spanned_by(value, err!(KeylenWithSentinel));
        }
        // --------------------------------------------------
        // a zero-byte length prefix has no room for a length: the container's or the sentinel's
        // own
        // --------------------------------------------------
        let len = match &cont.headers.len {
            CodecAttr::Parsed(_, len) => Some(len),
            CodecAttr::Absent | CodecAttr::Invalid(_) => None,
        };
        let own = cont
            .sentinel
            .as_ref()
            .and_then(|sentinel| sentinel.len.as_ref());
        for len in len.into_iter().chain(own.map(|(_, own)| own)) {
            if let Some((0, exact)) = len.size.exact() {
                cx.error_spanned_by(exact, err!(LenZeroWidth));
            }
        }
        // --------------------------------------------------
        // `unknown(..)` decodes a frame whose key no field declares, which `deny_unknown_keys`
        // rejects
        // --------------------------------------------------
        if cont.unknown.is_present()
            && let Some(deny) = &cont.deny_unknown_keys
        {
            cx.error_spanned_by(deny, err!(UnknownWithDenyUnknownKeys));
        }
        // --------------------------------------------------
        // resolve the framing mode: `keylen` and `key`/`len` are mutually exclusive,
        // and exactly one side must be fully specified
        // --------------------------------------------------
        let mode = match cont.headers {
            // --------------------------------------------------
            // classic mode, both parsed
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Absent,
                key: CodecAttr::Parsed(_, key),
                len: CodecAttr::Parsed(_, len),
            } => ContainerMode::Separate {
                key,
                len,
                sentinel: cont.sentinel.map(|Sentinel { value, len }| Sentinel {
                    value,
                    len: len.map(|(_, len)| len),
                }),
            },
            // --------------------------------------------------
            // classic mode, one half missing
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Absent,
                key,
                len: CodecAttr::Absent,
            } if key.is_present() => {
                cx.error_spanned_by(name, err!(MissingLength));
                return None;
            }
            HeaderAttrs {
                keylen: CodecAttr::Absent,
                key: CodecAttr::Absent,
                len,
            } if len.is_present() => {
                cx.error_spanned_by(name, err!(MissingKey));
                return None;
            }
            // --------------------------------------------------
            // nothing specified
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Absent,
                key: CodecAttr::Absent,
                len: CodecAttr::Absent,
            } => {
                cx.error_spanned_by(name, err!(MissingFraming));
                return None;
            }
            // --------------------------------------------------
            // an unusable attribute that already reported its error: classic mode with both
            // halves present and at least one unusable, or an unusable `keylen` alone
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Absent,
                ..
            }
            | HeaderAttrs {
                keylen: CodecAttr::Invalid(_),
                key: CodecAttr::Absent,
                len: CodecAttr::Absent,
            } => {
                return None;
            }
            // --------------------------------------------------
            // fused codec, no separate key/len; a `sentinel` without its own `len(..)` was
            // reported above
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Parsed(_, keylen),
                key: CodecAttr::Absent,
                len: CodecAttr::Absent,
            } => {
                let sentinel = match cont.sentinel {
                    Some(Sentinel {
                        value,
                        len: Some((_, len)),
                    }) => Some(Sentinel { value, len }),
                    Some(Sentinel { len: None, .. }) => return None,
                    None => None,
                };
                ContainerMode::Fused { keylen, sentinel }
            }
            // --------------------------------------------------
            // fused codec alongside any separate key/len, usable or not: conflict
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Invalid(path) | CodecAttr::Parsed(path, _),
                ..
            } => {
                cx.error_spanned_by(&path, err!(KeylenWithKeyOrLen));
                return None;
            }
        };
        // --------------------------------------------------
        // return parsed container, and the unknown-frame codec beside either mode
        // --------------------------------------------------
        let parsed = ContainerParsed {
            stream: cont.stream,
            mode,
            break_on: cont.break_on,
            debug: cont.debug,
            deny_unknown_keys: cont.deny_unknown_keys,
            strict: cont.strict,
        };
        Some((parsed, cont.unknown))
    }

    /// Reports a `sentinel` on a `&str` stream: the decode searches for it in the input's bytes,
    /// which a `&str` stream does not give
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the error
    pub(super) fn report_sentinel_on_str_stream(&self, cx: &Ctxt) {
        if let Some((sentinel, _)) = self.mode.packet()
            && let Some(syn::Type::Reference(stream)) = &self.stream
            && let syn::Type::Path(referent) = &*stream.elem
            && referent.path.is_ident("str")
        {
            cx.error_spanned_by(sentinel, err!(SentinelOnStrStream));
        }
    }
}
