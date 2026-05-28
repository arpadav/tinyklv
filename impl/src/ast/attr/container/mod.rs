//! Parsing of container-level `#[klv(..)]` attributes
//!
//! Defines [`Container`] (the raw parsed form of every recognised attribute on
//! a `#[derive(Klv)]` struct) and [`ContainerParsed`] (the validated form that
//! guarantees `key` and `len` are both present). Sub-modules handle the
//! individual attribute parsers: `default`, `keylen`, `sentinel`, and `stream`
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod default;
pub(crate) mod keylen;
pub(crate) mod sentinel;
pub(crate) mod stream;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::symbol;
use crate::Ctxt;
use default::DefaultXcoder;
use keylen::Xcoder;
use sentinel::Sentinel;
use stream::Stream;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;
use syn::{punctuated::Punctuated, Token};

#[derive(Debug)]
/// Raw parsed form of every recognised container-level `#[klv(..)]` attribute
///
/// All fields are `Option` because the attribute parser accumulates whatever it
/// finds and leaves validation to [`ContainerParsed::from_cont`]. Fields that
/// must be present for a valid derive (e.g. `key`, `len`) will produce errors
/// in the [`Ctxt`] if absent
pub(crate) struct Container {
    /// The stream type used for decoding, e.g. `&[u8]` (defaults when absent)
    pub stream: Option<syn::Type>,

    /// The recognition sentinel literal, e.g. `b"HEARTBEAT"`
    pub sentinel: Option<syn::Lit>,

    /// The key encoder/decoder xcoder pair
    pub key: Option<Xcoder>,

    /// The length encoder/decoder xcoder pair
    pub len: Option<Xcoder>,

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
        let mut key = None;
        let mut len = None;
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
                        symbol::KEY => match key {
                            Some(_) => cx.error_spanned_by(&list, err!(DuplicateKey)),
                            None => {
                                key = Xcoder::try_from(&list)
                                    .map_err(|err| cx.syn_error(err))
                                    .ok();
                            }
                        },

                        symbol::LENGTH => match len {
                            Some(_) => cx.error_spanned_by(&list.path, err!(DuplicateLength)),
                            None => {
                                len = Xcoder::try_from(&list)
                                    .map_err(|err| cx.syn_error(err))
                                    .ok();
                            }
                        },

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
                                stream = Stream::try_from(&nv).map_err(|err| cx.syn_error(err)).ok();
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
                        // --------------------------------------------------
                        // non name-values
                        // --------------------------------------------------
                        symbol::KEY => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::KEY)));
                        }
                        symbol::LENGTH => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::LENGTH)));
                        }
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
                        symbol::KEY => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::KEY)));
                        }
                        symbol::LENGTH => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::LENGTH)));
                        }
                        symbol::DEFAULT => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::DEFAULT)));
                        }
                        symbol::STREAM => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::STREAM)));
                        }
                        symbol::SENTINEL => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::SENTINEL)));
                        }
                        _ => cx.error_spanned_by(&path, err!(UnknownContainerAttribute(path))),
                    },
                }
            }
        }

        // --------------------------------------------------
        // unimplemented encode error
        // --------------------------------------------------
        if allow_unimplemented_encode.is_none() {
            if let Some(Xcoder { enc: None, .. }) = key {
                cx.error_spanned_by(&item.ident, err!(MissingEncInKeyLen(symbol::KEY)));
            }
            if let Some(Xcoder { enc: None, .. }) = len {
                cx.error_spanned_by(&item.ident, err!(MissingEncInKeyLen(symbol::LENGTH)));
            }
        }
        // --------------------------------------------------
        // unimplemented decode error
        // --------------------------------------------------
        if allow_unimplemented_decode.is_none() {
            if let Some(Xcoder { dec: None, .. }) = key {
                cx.error_spanned_by(&item.ident, err!(MissingDecInKeyLen(symbol::KEY)));
            }
            if let Some(Xcoder { dec: None, .. }) = len {
                cx.error_spanned_by(&item.ident, err!(MissingDecInKeyLen(symbol::LENGTH)));
            }
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Container {
            stream: stream.and_then(|x| x.0),
            sentinel: sentinel.and_then(|x| x.0),
            key,
            len,
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
/// verifies that both `key` and `len` xcoders are present and emits compile
/// errors via [`Ctxt`] if either is missing. The underscore-prefixed fields
/// (`_allow_unimplemented_decode`, `_allow_unimplemented_encode`,
/// `_trait_fallback`) are not consumed during expansion but are retained for
/// potential future diagnostics or tooling
pub(crate) struct ContainerParsed {
    /// The stream type used for decoding; `None` means `&[u8]` will be used
    pub stream: Option<syn::Type>,

    /// The recognition sentinel literal, if any
    pub sentinel: Option<syn::Lit>,

    /// The key xcoder (always present after validation)
    pub key: Xcoder,

    /// The length xcoder (always present after validation)
    pub len: Xcoder,

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
    /// Checks that both `key` and `len` xcoders are present, records errors
    /// in `cx` for any that are missing, and returns `None` if validation fails
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording missing-field diagnostics
    /// * `name` - The struct ident, used as the span anchor for error messages
    /// * `cont` - The raw parsed container attributes to validate
    ///
    /// # Returns
    ///
    /// `Some(ContainerParsed)` when both `key` and `len` are present, or `None`
    /// if either is absent (errors are recorded in `cx`)
    pub fn from_cont(cx: &Ctxt, name: &syn::Ident, cont: Container) -> Option<Self> {
        // --------------------------------------------------
        // check for required fields
        // --------------------------------------------------
        let (key, len) = match (cont.key, cont.len) {
            (Some(k), Some(l)) => (k, l),
            (Some(_), None) => {
                cx.error_spanned_by(name, err!(MissingLength));
                return None;
            }
            (None, Some(_)) => {
                cx.error_spanned_by(name, err!(MissingKey));
                return None;
            }
            (None, None) => {
                cx.error_spanned_by(name, err!(MissingKey));
                cx.error_spanned_by(name, err!(MissingLength));
                return None;
            }
        };
        // --------------------------------------------------
        // return parsed container
        // --------------------------------------------------
        Some(ContainerParsed {
            stream: cont.stream,
            sentinel: cont.sentinel,
            key,
            len,
            debug: cont.debug,
            deny_unknown_keys: cont.deny_unknown_keys,
            _allow_unimplemented_decode: cont.allow_unimplemented_decode,
            _allow_unimplemented_encode: cont.allow_unimplemented_encode,
            _trait_fallback: cont.trait_fallback,
        })
    }
}
