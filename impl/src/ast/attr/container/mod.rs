//! Parsing of container-level `#[klv(..)]` attributes
//!
//! Defines [`Container`] (the raw parsed form of every recognised attribute on
//! a `#[derive(Klv)]` struct) and [`ContainerParsed`] (the validated form that
//! resolves exactly one framing mode, [`ContainerMode`]). Sub-modules handle the
//! individual attribute parsers: `break_on`, `default`, `header` (the `key`, `len` and `keylen`
//! lists), `sentinel` and `stream`
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod break_on;
pub(crate) mod default;
mod header;
mod sentinel;
mod stream;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{Ctxt, symbol};
use break_on::{BreakOn, BreakOnSpec};
use default::DefaultXcoder;
use header::{HeaderSlot, HeaderXcoder, HeaderXcoderAttr};
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
/// validation. The `expand` pass picks the decode loop's frame header and body
/// from it; a fused container generates decode only
#[derive(Debug, Clone)]
pub(crate) enum ContainerMode {
    /// Classic mode: separate `key`/`len` xcoders; every value is sliced via `take(len)`
    Separate {
        /// The key xcoder
        key: Box<HeaderXcoder>,
        /// The length xcoder
        len: Box<HeaderXcoder>,
    },
    /// Fused mode: one `keylen` codec returning `(key, Framing)` so framing can depend on the key
    Fused(Box<HeaderXcoder>),
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

    /// The key encoder/decoder xcoder pair: absent, present but unusable, or parsed
    key: HeaderXcoderAttr,

    /// The length encoder/decoder xcoder pair: absent, present but unusable, or parsed
    len: HeaderXcoderAttr,

    /// The fused key+length codec (mutually exclusive with `key`/`len`): absent, present but
    /// unusable, or parsed
    keylen: HeaderXcoderAttr,

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
        let mut key = HeaderXcoderAttr::Absent;
        let mut len = HeaderXcoderAttr::Absent;
        let mut keylen = HeaderXcoderAttr::Absent;
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
                match meta {
                    // --------------------------------------------------
                    // handle all: `syn::MetaList`
                    // e.g. `key(enc = <..>, dec = <..>),`
                    // --------------------------------------------------
                    syn::Meta::List(list) => match symbol::Symbol::from(&list.path) {
                        symbol::KEY => key.on_list(cx, &list, HeaderSlot::Key),
                        symbol::LENGTH => len.on_list(cx, &list, HeaderSlot::Length),
                        symbol::KEYLEN => keylen.on_list(cx, &list, HeaderSlot::Keylen),

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
                    syn::Meta::NameValue(nv) => match symbol::Symbol::from(&nv.path) {
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
                        symbol::KEY => key.on_non_list(cx, &nv.path, HeaderSlot::Key),
                        symbol::LENGTH => len.on_non_list(cx, &nv.path, HeaderSlot::Length),
                        symbol::KEYLEN => keylen.on_non_list(cx, &nv.path, HeaderSlot::Keylen),
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
                    syn::Meta::Path(path) => match symbol::Symbol::from(&path) {
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
                        symbol::KEY => key.on_non_list(cx, &path, HeaderSlot::Key),
                        symbol::LENGTH => len.on_non_list(cx, &path, HeaderSlot::Length),
                        symbol::KEYLEN => keylen.on_non_list(cx, &path, HeaderSlot::Keylen),
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
        // `enc`/`dec` is missing
        // --------------------------------------------------
        for attr in [&key, &len, &keylen] {
            let HeaderXcoderAttr::Parsed(path, xcoder) = attr else {
                continue;
            };
            if allow_unimplemented_encode.is_none() && xcoder.enc.is_none() {
                cx.error_spanned_by(path, err!(MissingEncInHeader(path)));
            }
            if allow_unimplemented_decode.is_none() && xcoder.dec.is_none() {
                cx.error_spanned_by(path, err!(MissingDecInHeader(path)));
            }
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Container {
            stream: stream.and_then(|x| x.0),
            sentinel: sentinel.and_then(|x| x.0),
            break_on: break_on.and_then(|x| x.0),
            key,
            len,
            keylen,
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

    /// The recognition sentinel literal, if any
    pub sentinel: Option<syn::Lit>,

    /// The break condition, if any: a key literal (stop on match) or a `fn(key, len) -> BreakType`
    pub break_on: Option<BreakOnSpec>,

    /// The validated key/length framing mode (classic separate or fused)
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
    /// absent cases, and for `sentinel` or `break_on` on a fused container, and
    /// returns `None` if validation fails. The `sentinel`/`break_on` checks run
    /// before any early return, so they surface in the same compile as a framing
    /// conflict. An attribute that is present but unusable (it failed to parse,
    /// or was not written as a list) counts as present: its own error is not
    /// joined by a missing-`key`/`len`/framing one, and a `keylen` beside any
    /// `key`/`len` still reports the conflict
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording missing-field diagnostics
    /// * `name` - The struct ident, used as the span anchor for error messages
    /// * `cont` - The raw parsed container attributes to validate
    ///
    /// # Returns
    ///
    /// `Some(ContainerParsed)` when exactly one framing mode is fully specified,
    /// or `None` otherwise (errors are recorded in `cx`)
    pub fn from_cont(cx: &Ctxt, name: &syn::Ident, cont: Container) -> Option<Self> {
        // --------------------------------------------------
        // `sentinel` / `break_on` beside any `keylen(..)`, reported before the framing
        // resolution can return early
        // --------------------------------------------------
        let fused_conflict =
            cont.keylen.is_present() && (cont.sentinel.is_some() || cont.break_on.is_some());
        if fused_conflict {
            if let Some(sentinel) = &cont.sentinel {
                cx.error_spanned_by(sentinel, err!(KeylenWithSentinel));
            }
            if let Some(break_on) = &cont.break_on {
                cx.error_spanned_by(break_on, err!(KeylenWithBreakOn));
            }
        }
        // --------------------------------------------------
        // resolve the framing mode: `keylen` and `key`/`len` are mutually exclusive,
        // and exactly one side must be fully specified
        // --------------------------------------------------
        let mode = match (cont.keylen, cont.key, cont.len) {
            // --------------------------------------------------
            // classic mode, both parsed
            // --------------------------------------------------
            (
                HeaderXcoderAttr::Absent,
                HeaderXcoderAttr::Parsed(_, key),
                HeaderXcoderAttr::Parsed(_, len),
            ) => ContainerMode::Separate { key, len },
            // --------------------------------------------------
            // classic mode, one half missing
            // --------------------------------------------------
            (HeaderXcoderAttr::Absent, key, HeaderXcoderAttr::Absent) if key.is_present() => {
                cx.error_spanned_by(name, err!(MissingLength));
                return None;
            }
            (HeaderXcoderAttr::Absent, HeaderXcoderAttr::Absent, len) if len.is_present() => {
                cx.error_spanned_by(name, err!(MissingKey));
                return None;
            }
            // --------------------------------------------------
            // nothing specified
            // --------------------------------------------------
            (HeaderXcoderAttr::Absent, HeaderXcoderAttr::Absent, HeaderXcoderAttr::Absent) => {
                cx.error_spanned_by(name, err!(MissingFraming));
                return None;
            }
            // --------------------------------------------------
            // an unusable attribute that already reported its error: classic mode with both
            // halves present and at least one unusable, or an unusable `keylen` alone
            // --------------------------------------------------
            (HeaderXcoderAttr::Absent, _, _)
            | (HeaderXcoderAttr::Invalid(_), HeaderXcoderAttr::Absent, HeaderXcoderAttr::Absent) => {
                return None;
            }
            // --------------------------------------------------
            // fused codec, no separate key/len
            // --------------------------------------------------
            (
                HeaderXcoderAttr::Parsed(_, keylen),
                HeaderXcoderAttr::Absent,
                HeaderXcoderAttr::Absent,
            ) => {
                if fused_conflict {
                    return None;
                }
                ContainerMode::Fused(keylen)
            }
            // --------------------------------------------------
            // fused codec alongside any separate key/len, usable or not: conflict
            // --------------------------------------------------
            (HeaderXcoderAttr::Invalid(path) | HeaderXcoderAttr::Parsed(path, _), _, _) => {
                cx.error_spanned_by(&path, err!(KeylenWithKeyOrLen));
                return None;
            }
        };
        // --------------------------------------------------
        // return parsed container
        // --------------------------------------------------
        Some(ContainerParsed {
            stream: cont.stream,
            sentinel: cont.sentinel,
            break_on: cont.break_on,
            mode,
            debug: cont.debug,
            deny_unknown_keys: cont.deny_unknown_keys,
            _allow_unimplemented_decode: cont.allow_unimplemented_decode,
            _allow_unimplemented_encode: cont.allow_unimplemented_encode,
            _trait_fallback: cont.trait_fallback,
        })
    }
}
