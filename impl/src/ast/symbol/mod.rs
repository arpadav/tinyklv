#![allow(clippy::expect_used, reason = "proc macro okay to panic")]
//! Symbol definitions and comparison helpers for KLV attribute parsing
//!
//! Defines the [`Symbol`] type (a wrapper around a `&'static str`) used to
//! identify attribute names in `#[klv(..)]` annotations, along with the
//! [`Symbols`] collection type used to display valid symbol sets in error
//! messages. All known symbol constants and static symbol-set arrays are
//! declared here
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod parsers;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
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
/// The `size` sub-attribute identifier for `key(..)`/`len(..)`/fields; see
/// [`SizeSpec`](crate::ast::attr::size::SizeSpec)
pub(crate) const SIZE: Symbol = Symbol("size");
/// The `var` inner keyword of `size(..)`
pub(crate) const VAR: Symbol = Symbol("var");
/// The `exact = N` inner keyword of `size(..)`
pub(crate) const EXACT: Symbol = Symbol("exact");
/// The `hint = N` inner keyword of `size(..)`
pub(crate) const HINT: Symbol = Symbol("hint");
/// The `stream` sub-attribute identifier
pub(crate) const STREAM: Symbol = Symbol("stream");
/// The `default` sub-attribute identifier
pub(crate) const DEFAULT: Symbol = Symbol("default");
/// The `sentinel` sub-attribute identifier
pub(crate) const SENTINEL: Symbol = Symbol("sentinel");
/// The `break_on` container sub-attribute identifier
///
/// Accepts either a key literal (a decoded key equal to it stops the loop -
/// [`tinyklv::BreakType::Done`]) or a function path `fn(key, len) -> BreakType`
pub(crate) const BREAK_ON: Symbol = Symbol("break_on");
/// The field-level `default` sub-attribute identifier
///
/// Note: the container-level `default(..)` uses the same keyword [`DEFAULT`]
/// above; they are parsed in different `MetaList` scopes so there is no
/// collision
pub(crate) const DEFAULT_VALUE: Symbol = Symbol("default");
/// The `latebind` sub-attribute identifier for post-decode conversion/mutation
pub(crate) const LATEBIND: Symbol = Symbol("latebind");
/// The `deny_unknown_keys` sub-attribute identifier
pub(crate) const DENY_UNKNOWN_KEYS: Symbol = Symbol("deny_unknown_keys");
/// The `allow_unimplemented_decode` sub-attribute identifier
pub(crate) const ALLOW_UNIMPLEMENTED_DECODE: Symbol = Symbol("allow_unimplemented_decode");
/// The `allow_unimplemented_encode` sub-attribute identifier
pub(crate) const ALLOW_UNIMPLEMENTED_ENCODE: Symbol = Symbol("allow_unimplemented_encode");
/// The `trait_fallback` sub-attribute identifier
///
/// Opt-in container flag. When set, any field lacking an explicit `enc`/`dec`
/// and not matched by a container `default(..)` falls back to the
/// [`tinyklv::EncodeValue`] / [`tinyklv::DecodeValue`] trait implementations
/// for the field's type. If the trait is not implemented for the field's type,
/// the compiler reports a trait-bound error at the call site
pub(crate) const TRAIT_FALLBACK: Symbol = Symbol("trait_fallback");

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
    BREAK_ON,
    DENY_UNKNOWN_KEYS,
    ALLOW_UNIMPLEMENTED_DECODE,
    ALLOW_UNIMPLEMENTED_ENCODE,
    TRAIT_FALLBACK,
]);

/// Container-level symbols that accept list syntax (e.g. `key(..)`)
pub(crate) static CONT_LIST_SYMBOLS: Symbols = Symbols(&[KEY, LENGTH, DEFAULT]);

/// Container-level default list symbols (type, encoder, decoder, value size)
pub(crate) static CONT_DEFAULT_LIST_SYMBOLS: Symbols = Symbols(&[TYPE, ENCODER, DECODER, SIZE]);

/// Container-level name-value symbols (e.g. `stream = ..`)
pub(crate) static CONT_NV_SYMBOLS: Symbols = Symbols(&[STREAM, SENTINEL, BREAK_ON]);

/// All valid field-level symbols accepted by the `#[klv(..)]` attribute
pub(crate) static FIELD_SYMBOLS: Symbols =
    Symbols(&[KEY, ENCODER, DECODER, SIZE, LATEBIND, DEFAULT_VALUE]);

/// Valid inner keywords of the `size(..)` list (`var`, `exact = N`, `hint = N`)
pub(crate) static SIZE_FIELDS: Symbols = Symbols(&[VAR, EXACT, HINT]);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// A symbol for KLV attributes
pub(crate) struct Symbol(pub(crate) &'static str);

/// [`Symbol`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Sentinel [`Symbol`] returned for any path that is not one of the known
/// KLV attribute identifiers. Callers match against the known constants first,
/// and fall through to a `_` arm that emits an error referencing the original
/// `syn::Path` - so the sentinel's string is never displayed
pub(crate) const UNKNOWN: Symbol = Symbol("<unknown>");

/// All known [`Symbol`] constants that [`Symbol::from`] can resolve against
/// Lookup is linear - the set is tiny (<20) and resolution runs at macro-expansion time
const KNOWN: &[Symbol] = &[
    KLV_ATTR,
    KEY,
    TYPE,
    DEBUG,
    LENGTH,
    ENCODER,
    DECODER,
    SIZE,
    VAR,
    EXACT,
    HINT,
    STREAM,
    DEFAULT,
    SENTINEL,
    BREAK_ON,
    DEFAULT_VALUE,
    LATEBIND,
    DENY_UNKNOWN_KEYS,
    ALLOW_UNIMPLEMENTED_DECODE,
    ALLOW_UNIMPLEMENTED_ENCODE,
    TRAIT_FALLBACK,
];

/// [`Symbol`] implementation of [`From`] for [`syn::Path`]
///
/// Resolves the path's last segment against [`KNOWN`]. If the ident matches a known
/// KLV symbol, returns that constant (with its `&'static str` backing). Otherwise
/// returns [`UNKNOWN`] - callers must treat this as the "default" match arm and
/// emit errors referencing the original `syn::Path`, not the [`Symbol`]'s string
impl From<&syn::Path> for Symbol {
    fn from(path: &syn::Path) -> Self {
        let last = path.segments.last().expect("path has no segments");
        for sym in KNOWN {
            if last.ident == sym.0 {
                return *sym;
            }
        }
        UNKNOWN
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
        tokens.extend(quote::quote! { #ident });
    }
}

/// [`syn::Path`] implementation of [`PartialEq`] against [`Symbol`]
///
/// Allows writing `attr.path() == symbol::KEY` in match guards and `if` conditions
impl PartialEq<Symbol> for syn::Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

/// `&syn::Path` implementation of [`PartialEq`] against [`Symbol`]
///
/// Convenience impl so borrowed paths can be compared to [`Symbol`] constants
/// without an explicit dereference
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
