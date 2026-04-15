// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::parse::Parser;

// --------------------------------------------------
// local
// --------------------------------------------------
mod parsers;
pub(crate) use parsers::*;

// All symbols (all idents, e.g. no kebab-case)
pub(crate) const KLV_ATTR: Symbol = Symbol(&crate::ATTR_NAME);
pub(crate) const KEY: Symbol = Symbol("key");
pub(crate) const TYPE: Symbol = Symbol("typ");
pub(crate) const DEBUG: Symbol = Symbol("debug");
pub(crate) const LENGTH: Symbol = Symbol("len");
pub(crate) const ENCODER: Symbol = Symbol("enc");
pub(crate) const DECODER: Symbol = Symbol("dec");
pub(crate) const STREAM: Symbol = Symbol("stream");
pub(crate) const DEFAULT: Symbol = Symbol("default");
pub(crate) const SENTINEL: Symbol = Symbol("sentinel");
pub(crate) const INITIAL_VALUE: Symbol = Symbol("init");
pub(crate) const VARIABLE_LENGTH: Symbol = Symbol("var");
pub(crate) const DENY_UNKNOWN_KEYS: Symbol = Symbol("deny_unknown_keys");
pub(crate) const ALLOW_LENGTH_MISMATCH: Symbol = Symbol("allow_length_mismatch");
pub(crate) const ALLOW_UNIMPLEMENTED_DECODE: Symbol = Symbol("allow_unimplemented_decode");
pub(crate) const ALLOW_UNIMPLEMENTED_ENCODE: Symbol = Symbol("allow_unimplemented_encode");

/// Container symbols
pub(crate) static CONT_SYMBOLS: Symbols = Symbols(&[
    KEY,
    LENGTH,
    STREAM,
    DEFAULT,
    SENTINEL,
    DENY_UNKNOWN_KEYS,
    ALLOW_LENGTH_MISMATCH,
    ALLOW_UNIMPLEMENTED_DECODE,
    ALLOW_UNIMPLEMENTED_ENCODE,
]);

/// Container list symbols
pub(crate) static CONT_LIST_SYMBOLS: Symbols = Symbols(&[KEY, LENGTH, DEFAULT]);

/// Container default list symbols
pub(crate) static CONT_DEFAULT_LIST_SYMBOLS: Symbols =
    Symbols(&[TYPE, ENCODER, DECODER, VARIABLE_LENGTH]);

/// Container name-value symbols
pub(crate) static CONT_NV_SYMBOLS: Symbols = Symbols(&[STREAM, SENTINEL]);

/// Field symbols
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

// /// [`Symbol`] implementation of [`std::fmt::Debug`]
// impl std::fmt::Debug for Symbol {
//     fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str(self.0)
//     }
// }
/// [`Symbol`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}
/// [`Symbol`] implementation of [`From`] for [`syn::Path`]
impl From<&syn::Path> for Symbol {
    fn from(path: &syn::Path) -> Self {
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

// impl PartialEq<Symbol> for syn::Ident {
//     fn eq(&self, word: &Symbol) -> bool {
//         self == word.0
//     }
// }

// impl PartialEq<Symbol> for &syn::Ident {
//     fn eq(&self, word: &Symbol) -> bool {
//         *self == word.0
//     }
// }

/// Multiple symbols, for displaying errors
pub(crate) struct Symbols<'a>(&'a [Symbol]);

/// [`Symbols`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for Symbols<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut symbols = self.0.to_vec();
        // reverse alphabetical
        symbols.sort_by(|a, b| b.0.cmp(a.0));
        symbols
            .iter()
            .try_for_each(|symbol| write!(f, "`{}` ", symbol.0))
    }
}
