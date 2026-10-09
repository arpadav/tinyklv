//! Parsing of container-level `#[klv(..)]` attributes
//!
//! Defines [`Container`] (the raw parsed form of every recognised attribute on
//! a `#[derive(Klv)]` struct) and [`ContainerParsed`] (the validated form that
//! resolves exactly one framing mode, [`ContainerMode`]). Sub-modules handle the
//! individual attribute parsers: `break_on`, `codec` (the state of every codec attribute),
//! `default`, `header` (the `key` and `len` lists, and the three frame header states), `keylen`
//! (the fused `keylen` list), `sentinel`, `stream` and `unknown` (the codec of the frames whose
//! key no field declares)
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
use crate::{Ctxt, symbol};
use break_on::BreakOn;
use codec::CodecSlot;
use header::{HeaderAttrs, HeaderXcoder};
use keylen::KeylenXcoder;
use sentinel::Sentinel;
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
/// validation, and each mode holds only the `sentinel` and `break_on` forms it supports. The
/// `expand` pass picks the decode loop's frame header, `break_on` check and body, the frame
/// header written around each encoded value, and the sentinel's packet framing, from it
#[derive(Debug, Clone)]
pub(crate) enum ContainerMode {
    /// Classic mode: separate `key`/`len` xcoders; a value with a length prefix is sliced via
    /// `take(len)`
    Separate {
        /// The key xcoder
        key: Box<HeaderXcoder>,
        /// The length xcoder
        len: Box<HeaderXcoder>,
        /// The recognition sentinel literal, if any, e.g. `b"HEARTBEAT"`
        sentinel: Option<syn::Lit>,
        /// The break condition, if any: a key literal or a `fn(key, len) -> BreakType` path
        break_on: Option<BreakOnSpec>,
    },
    /// Fused mode: one `keylen` codec, whose decoder returns `(key, Framing)` so framing can
    /// depend on the key, and whose encoder writes the header from `(key, value_len)`. It has no
    /// sentinel: there is no standalone length codec to frame the packet with
    Fused {
        /// The fused key+length codec
        keylen: Box<KeylenXcoder>,
        /// The key literal that stops the loop, if any; a `break_on` fn would need a length an
        /// `Inline` frame does not have
        break_on: Option<syn::Lit>,
    },
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

    /// The recognition sentinel literal, e.g. `b"HEARTBEAT"`
    pub sentinel: Option<syn::Lit>,

    /// The break condition: a key literal (stop on match) or a `fn(key, len) -> BreakType` path
    pub break_on: Option<BreakOnSpec>,

    /// The key and length xcoder pairs and the fused key+length codec (mutually exclusive with
    /// `key`/`len`): each absent, present but unusable, or parsed
    headers: HeaderAttrs,

    /// The codec of the frames whose key no field declares: absent, present but unusable, or
    /// parsed
    unknown: CodecAttr<UnknownXcoder>,

    /// Per-type default xcoders keyed by the Rust type they apply to
    pub defaults: HashMap<syn::Type, DefaultXcoder>,

    /// Whether debug logging is enabled for this container
    pub debug: Option<syn::Path>,

    /// Whether unknown keys cause a decode error rather than being silently skipped
    pub deny_unknown_keys: Option<syn::Path>,

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
        let mut defaults: HashMap<syn::Type, DefaultXcoder> = HashMap::new();
        let mut debug = None;
        let mut deny_unknown_keys = None;
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
                match meta {
                    // --------------------------------------------------
                    // handle all: `syn::MetaList`
                    // e.g. `key(enc = <..>, dec = <..>),`
                    // --------------------------------------------------
                    syn::Meta::List(list) => match name {
                        symbol::DEFAULT => {
                            let new = DefaultXcoder::from(&list);
                            if let Some(err) = new.errors {
                                cx.syn_error(err);
                                continue;
                            }
                            // this should always be `Some` otherwise `new.errors` would have
                            // been `Some` and caught above. but I don't want to unwrap
                            let Some(new_typ) = new.typ.clone() else {
                                continue;
                            };

                            match defaults.get_mut(&new_typ) {
                                Some(old) => {
                                    match (&old.dec, &new.dec) {
                                        (Some(_), Some(dec_path)) => cx.error_spanned_by(
                                            dec_path,
                                            err!(DuplicateDefault(new.typ; "decoder")),
                                        ),
                                        (None, _) => old.dec = new.dec,
                                        _ => (),
                                    }
                                    match (&old.enc, &new.enc) {
                                        (Some(_), Some(enc_path)) => cx.error_spanned_by(
                                            enc_path,
                                            err!(DuplicateDefault(new.typ; "encoder")),
                                        ),
                                        (None, _) => old.enc = new.enc,
                                        _ => (),
                                    }
                                    match (&old.size, &new.size) {
                                        (Some(_), Some(size)) => cx.syn_error(syn::Error::new(
                                            size.span(),
                                            err!(DuplicateDefault(new.typ; "size")),
                                        )),
                                        (None, _) => old.size = new.size,
                                        _ => (),
                                    }
                                    match (&old.skip_if, &new.skip_if) {
                                        (Some(_), Some(skip_if)) => {
                                            cx.syn_error(syn::Error::new(
                                                skip_if.span(),
                                                err!(DuplicateDefault(new.typ; "skip")),
                                            ));
                                        }
                                        (None, _) => old.skip_if = new.skip_if,
                                        _ => (),
                                    }
                                }

                                None => {
                                    let _ = defaults.insert(new_typ, new);
                                }
                            }
                        }
                        // --------------------------------------------------
                        // non lists
                        // --------------------------------------------------
                        symbol::STREAM => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsNameValue(symbol::STREAM)),
                        ),
                        symbol::SENTINEL => cx.error_spanned_by(
                            &list.path,
                            err!(ExpectedAsNameValue(symbol::SENTINEL)),
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
                        symbol::SENTINEL => match sentinel {
                            Some(_) => cx.error_spanned_by(&nv, err!(DuplicateSentinel)),
                            None => {
                                sentinel = Sentinel::try_from(&nv)
                                    .map_err(|err| cx.syn_error(err))
                                    .ok();
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
                    syn::Meta::Path(path) => match name {
                        symbol::DEBUG => debug = Some(path),
                        symbol::DENY_UNKNOWN_KEYS => deny_unknown_keys = Some(path),
                        symbol::ALLOW_UNIMPLEMENTED_DECODE => {
                            allow_unimplemented_decode = Some(path);
                        }
                        symbol::ALLOW_UNIMPLEMENTED_ENCODE => {
                            allow_unimplemented_encode = Some(path);
                        }
                        symbol::TRAIT_FALLBACK => trait_fallback = Some(path),
                        // --------------------------------------------------
                        // non paths
                        // --------------------------------------------------
                        symbol::DEFAULT => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::DEFAULT)));
                        }
                        symbol::STREAM => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::STREAM)));
                        }
                        symbol::SENTINEL => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::SENTINEL)));
                        }
                        symbol::BREAK_ON => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::BREAK_ON)));
                        }
                        _ => cx.error_spanned_by(&path, err!(UnknownContainerAttribute(path))),
                    },
                }
            }
        }
        // --------------------------------------------------
        // unimplemented encode / decode errors, spanned on the `key`/`len`/`keylen` path whose
        // `enc`/`dec` is missing; `unknown(..)`'s are reported once the fields are read, since
        // its `enc` is needed only to re-emit the frames the `#[klv(unknown)]` field keeps
        // --------------------------------------------------
        headers.report_missing(
            cx,
            allow_unimplemented_encode.is_some(),
            allow_unimplemented_decode.is_some(),
        );
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Container {
            stream: stream.and_then(|x| x.0),
            sentinel: sentinel.and_then(|x| x.0),
            break_on: break_on.and_then(|x| x.0),
            headers,
            unknown,
            defaults,
            debug,
            deny_unknown_keys,
            allow_unimplemented_decode,
            allow_unimplemented_encode,
            trait_fallback,
        }
    }
}

/// Validated container attributes where required fields are guaranteed present
///
/// Produced from a [`Container`] by [`ContainerParsed::from_cont`], which
/// resolves the key/length framing into a [`ContainerMode`] - verifying that
/// exactly one of `key`+`len` or `keylen` is present and emitting compile
/// errors via [`Ctxt`] otherwise. The underscore-prefixed fields
/// (`_allow_unimplemented_decode`, `_allow_unimplemented_encode`,
/// `_trait_fallback`) are not consumed during expansion but are retained for
/// potential future diagnostics or tooling
pub(crate) struct ContainerParsed {
    /// The stream type used for decoding; `None` means `&[u8]` will be used
    pub stream: Option<syn::Type>,

    /// The validated key/length framing mode (classic separate or fused), with its `sentinel`
    /// and `break_on`
    pub mode: ContainerMode,

    /// Whether debug logging is enabled for this container
    pub debug: Option<syn::Path>,

    /// Whether unknown keys cause a decode error rather than being silently skipped
    pub deny_unknown_keys: Option<syn::Path>,

    /// Retained path for `allow_unimplemented_decode`, unused in expansion
    pub _allow_unimplemented_decode: Option<syn::Path>,

    /// Retained path for `allow_unimplemented_encode`, unused in expansion
    pub _allow_unimplemented_encode: Option<syn::Path>,

    /// Retained path for `trait_fallback`, unused in expansion
    pub _trait_fallback: Option<syn::Path>,
}
/// [`ContainerParsed`] implementation
impl ContainerParsed {
    /// Converts a raw [`Container`] into a validated [`ContainerParsed`]
    ///
    /// Resolves the key/length framing into a [`ContainerMode`]: exactly one of
    /// the separate `key`+`len` pair or the fused `keylen` codec must be
    /// present. Records errors in `cx` for the conflicting, half-specified, or
    /// absent cases and for `sentinel` or a `break_on` function on a fused container, and
    /// returns `None` for them. It also records an error for `deny_unknown_keys` beside
    /// `unknown(..)`, and still returns the container. The `sentinel`/`break_on`/`unknown`
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
    /// specified, and a fused one has no `sentinel` and no `break_on` function, or `None`
    /// otherwise (errors are recorded in `cx`)
    pub fn from_cont(
        cx: &Ctxt,
        name: &syn::Ident,
        cont: Container,
    ) -> Option<(Self, CodecAttr<UnknownXcoder>)> {
        // --------------------------------------------------
        // `sentinel` / a `break_on` function beside any `keylen(..)`, reported before the
        // framing resolution can return early
        // --------------------------------------------------
        if cont.headers.keylen.is_present() {
            if let Some(sentinel) = &cont.sentinel {
                cx.error_spanned_by(sentinel, err!(KeylenWithSentinel));
            }
            if let Some(BreakOnSpec::Func(path)) = &cont.break_on {
                cx.error_spanned_by(path, err!(KeylenWithBreakOnFn));
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
                sentinel: cont.sentinel,
                break_on: cont.break_on,
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
            // fused codec, no separate key/len; a `sentinel` or a `break_on` fn beside it was
            // reported above
            // --------------------------------------------------
            HeaderAttrs {
                keylen: CodecAttr::Parsed(_, keylen),
                key: CodecAttr::Absent,
                len: CodecAttr::Absent,
            } => match (&cont.sentinel, cont.break_on) {
                (None, None) => ContainerMode::Fused {
                    keylen,
                    break_on: None,
                },
                (None, Some(BreakOnSpec::Literal(lit))) => ContainerMode::Fused {
                    keylen,
                    break_on: Some(lit),
                },
                (Some(_), _) | (None, Some(BreakOnSpec::Func(_))) => return None,
            },
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
            debug: cont.debug,
            deny_unknown_keys: cont.deny_unknown_keys,
            _allow_unimplemented_decode: cont.allow_unimplemented_decode,
            _allow_unimplemented_encode: cont.allow_unimplemented_encode,
            _trait_fallback: cont.trait_fallback,
        };
        Some((parsed, cont.unknown))
    }
}
