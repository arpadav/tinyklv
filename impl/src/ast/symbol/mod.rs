//! Symbol definitions and comparison helpers for KLV attribute parsing
//!
//! Defines the [`Symbol`] type (a wrapper around a `&'static str`) used to
//! identify attribute names in `#[klv(..)]` annotations, along with the
//! [`Symbols`] collection type used to display valid symbol sets in error
//! messages. All known symbol constants and static symbol-set arrays are
//! declared here
//!
//! Author: aav
#![allow(clippy::expect_used, reason = "proc macro okay to panic")]
// --------------------------------------------------
// mods
// --------------------------------------------------
mod parsers;
pub(crate) use parsers::*;
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::parse::Parser;
// --------------------------------------------------
// constants
// --------------------------------------------------
/// The top-level KLV attribute name (e.g. `#[klv(..)]`)
pub(crate) const KLV_ATTR: Symbol = Symbol(crate::ATTR_NAME);

/// The `key` sub-attribute identifier
pub(crate) const KEY: Symbol = Symbol("key");

/// The `typ` sub-attribute identifier
pub(crate) const TYPE: Symbol = Symbol("typ");

/// The `debug` sub-attribute identifier
pub(crate) const DEBUG: Symbol = Symbol("debug");

/// The `len` sub-attribute identifier
pub(crate) const LENGTH: Symbol = Symbol("len");

/// The `enc` sub-attribute identifier
pub(crate) const ENCODER: Symbol = Symbol("enc");

/// The `dec` sub-attribute identifier
pub(crate) const DECODER: Symbol = Symbol("dec");

/// The `stream` sub-attribute identifier
pub(crate) const STREAM: Symbol = Symbol("stream");

/// The `default` sub-attribute identifier
pub(crate) const DEFAULT: Symbol = Symbol("default");

/// The `sentinel` sub-attribute identifier
pub(crate) const SENTINEL: Symbol = Symbol("sentinel");

/// The `init` sub-attribute identifier
pub(crate) const INITIAL_VALUE: Symbol = Symbol("init");

/// The `var` sub-attribute identifier for variable-length fields
pub(crate) const VARIABLE_LENGTH: Symbol = Symbol("var");

/// The `deny_unknown_keys` sub-attribute identifier
pub(crate) const DENY_UNKNOWN_KEYS: Symbol = Symbol("deny_unknown_keys");

/// The `allow_unimplemented_decode` sub-attribute identifier
pub(crate) const ALLOW_UNIMPLEMENTED_DECODE: Symbol = Symbol("allow_unimplemented_decode");

/// The `allow_unimplemented_encode` sub-attribute identifier
pub(crate) const ALLOW_UNIMPLEMENTED_ENCODE: Symbol = Symbol("allow_unimplemented_encode");
// --------------------------------------------------
// statics
// --------------------------------------------------
/// All valid container-level symbols accepted by the `#[klv(..)]` attribute
pub(crate) static CONT_SYMBOLS: Symbols = Symbols(&[
    KEY,
    LENGTH,
    STREAM,
    DEFAULT,
    SENTINEL,
    DENY_UNKNOWN_KEYS,
    ALLOW_UNIMPLEMENTED_DECODE,
    ALLOW_UNIMPLEMENTED_ENCODE,
]);

/// Container-level symbols that accept list syntax (e.g. `key = [...]`)
pub(crate) static CONT_LIST_SYMBOLS: Symbols = Symbols(&[KEY, LENGTH, DEFAULT]);

/// Container-level default list symbols (type, encoder, decoder, var-length)
pub(crate) static CONT_DEFAULT_LIST_SYMBOLS: Symbols =
    Symbols(&[TYPE, ENCODER, DECODER, VARIABLE_LENGTH]);

/// Container-level name-value symbols
pub(crate) static CONT_NV_SYMBOLS: Symbols = Symbols(&[STREAM, SENTINEL]);

/// All valid field-level symbols accepted by the `#[klv(..)]` attribute
pub(crate) static FIELD_SYMBOLS: Symbols = Symbols(&[
    KEY,
    // LENGTH,             // <-- TODO add
    ENCODER,
    DECODER,
    VARIABLE_LENGTH, // <-- TODO deprecate
]);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// A symbol for KLV attributes
pub(crate) struct Symbol(pub(crate) &'static str);

/// [`Symbol`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// [`Symbol`] implementation of [`From`] for [`syn::Path`]
impl From<&syn::Path> for Symbol {
    fn from(path: &syn::Path) -> Self {
        // --------------------------------------------------
        // extract the last path segment as the symbol name,
        // leak it so it satisfies the `&'static str` bound
        // --------------------------------------------------
        let ident = path
            .segments
            .last()
            .expect("path has no segments")
            .ident
            .to_string();
        Symbol(Box::leak(ident.into_boxed_str()))
    }
}

/// [`syn::Path`] implementation of [`From`] for [`Symbol`]
impl From<Symbol> for syn::Path {
    fn from(symbol: Symbol) -> Self {
        syn::parse_str(symbol.0).expect("?")
    }
}

/// [`Symbol`] implementation of [`ToTokens`]
impl ToTokens for Symbol {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        #[allow(clippy::unwrap_used)] // all symbols are idents
        let ident: syn::Ident = syn::parse_str(self.0).unwrap();
        tokens.extend(quote::quote! { #ident })
    }
}

impl PartialEq<Symbol> for syn::Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl PartialEq<Symbol> for &syn::Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

/// Multiple symbols, for displaying errors
pub(crate) struct Symbols<'a>(&'a [Symbol]);

/// [`Symbols`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for Symbols<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // --------------------------------------------------
        // sort symbols in reverse alphabetical order before
        // writing them to the formatter
        // --------------------------------------------------
        let mut symbols = self.0.to_vec();
        symbols.sort_by(|a, b| b.0.cmp(a.0));
        symbols
            .iter()
            .try_for_each(|symbol| write!(f, "`{}` ", symbol.0))
    }
}
