// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod default;
pub(crate) mod keylen;
pub(crate) mod sentinel;
pub(crate) mod stream;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::Token;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::symbol;
use crate::Ctxt;
use default::DefaultXcoder;
use keylen::Xcoder;
use sentinel::Sentinel;
use stream::Stream;

#[derive(Debug)]
/// Represents struct attribute information
pub(crate) struct Container {
    pub stream: Option<syn::Type>,
    pub sentinel: Option<syn::Lit>,
    pub key: Option<Xcoder>,
    pub len: Option<Xcoder>,
    pub defaults: HashMap<syn::Type, DefaultXcoder>,
    pub debug: Option<syn::Path>,
    pub deny_unknown_keys: Option<syn::Path>,
    pub allow_unimplemented_decode: Option<syn::Path>,
    pub allow_unimplemented_encode: Option<syn::Path>,
}
/// [`Container`] implementation
impl Container {
    /// Extract out the `#[klv(...)]` attributes from a container.
    ///
    /// Only container implemented is struct.
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
                                    .ok()
                            }
                        },

                        symbol::LENGTH => match len {
                            Some(_) => cx.error_spanned_by(&list.path, err!(DuplicateLength)),
                            None => {
                                len = Xcoder::try_from(&list)
                                    .map_err(|err| cx.syn_error(err))
                                    .ok()
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
                            cx.error_spanned_by(&list.path, err!(ExpectedAsPath(symbol::DEBUG)))
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
                                stream = Stream::try_from(&nv).map_err(|err| cx.syn_error(err)).ok()
                            }
                        },
                        symbol::SENTINEL => match sentinel {
                            Some(_) => cx.error_spanned_by(&nv, err!(DuplicateSentinel)),
                            None => {
                                sentinel = Sentinel::try_from(&nv)
                                    .map_err(|err| cx.syn_error(err))
                                    .ok()
                            }
                        },
                        // --------------------------------------------------
                        // non name-values
                        // --------------------------------------------------
                        symbol::KEY => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::KEY)))
                        }
                        symbol::LENGTH => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::LENGTH)))
                        }
                        symbol::DEFAULT => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsList(symbol::DEFAULT)))
                        }
                        symbol::DEBUG => {
                            cx.error_spanned_by(&nv.path, err!(ExpectedAsPath(symbol::DEBUG)))
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
                            allow_unimplemented_decode = Some(path)
                        }
                        symbol::ALLOW_UNIMPLEMENTED_ENCODE => {
                            allow_unimplemented_encode = Some(path)
                        }
                        // --------------------------------------------------
                        // non paths
                        // --------------------------------------------------
                        symbol::KEY => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::KEY)))
                        }
                        symbol::LENGTH => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::LENGTH)))
                        }
                        symbol::DEFAULT => {
                            cx.error_spanned_by(&path, err!(ExpectedAsList(symbol::DEFAULT)))
                        }
                        symbol::STREAM => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::STREAM)))
                        }
                        symbol::SENTINEL => {
                            cx.error_spanned_by(&path, err!(ExpectedAsNameValue(symbol::SENTINEL)))
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
        }
    }
}

/// A parsed container
///
/// * `_allow_unimplemented_decode`
/// * `_allow_unimplemented_encode`
///
/// are currently not used at this stage, but the paths are kept for potential
/// future docs/debugging during expansion.
pub(crate) struct ContainerParsed {
    pub stream: Option<syn::Type>,
    pub sentinel: Option<syn::Lit>,
    pub key: Xcoder,
    pub len: Xcoder,
    pub debug: Option<syn::Path>,
    pub _deny_unknown_keys: Option<syn::Path>,
    pub _allow_unimplemented_decode: Option<syn::Path>,
    pub _allow_unimplemented_encode: Option<syn::Path>,
}
/// [`ContainerParsed`] implementation
impl ContainerParsed {
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
            _deny_unknown_keys: cont.deny_unknown_keys,
            _allow_unimplemented_decode: cont.allow_unimplemented_decode,
            _allow_unimplemented_encode: cont.allow_unimplemented_encode,
        })
    }
}
