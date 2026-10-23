//! Parsing of the container-level `sentinel` attribute
//!
//! Defines [`Sentinel`], the bytes each packet starts with and the codec of each packet's length.
//! `sentinel = b".."` names the bytes alone, and the container's `len(..)` frames the packet;
//! `sentinel(value = b"..", len(..))` gives the packet length a codec of its own, its `len(..)`
//! parsed as a container's is ([`HeaderXcoder`]). When absent the container has no packets and
//! [`tinyklv::traits::SeekSentinel`] is not derived
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::header::HeaderXcoder;
use crate::ast::symbol;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// A container's `sentinel`: the bytes each packet starts with, and the codec of each packet's
/// length, an `L`
///
/// Parsed as a [`ParsedSentinel`]. The framing mode holds the codec alone: optional beside
/// `key(..)` and `len(..)`, whose `len(..)` frames the packet without one, and required beside
/// `keylen(..)`
#[derive(Debug, Clone)]
pub(crate) struct Sentinel<L> {
    /// The bytes each packet starts with, e.g. `b"HEARTBEAT"`
    pub(super) value: syn::Lit,

    /// The codec of each packet's length
    pub(super) len: L,
}

/// A [`Sentinel`] as parsed: the path of its own `len(..)`, which spans the codec's errors, and the
/// codec, if any
pub(super) type ParsedSentinel = Sentinel<Option<(syn::Path, Box<HeaderXcoder>)>>;

/// [`ParsedSentinel`] implementation of [`TryFrom`] for [`syn::Meta`]
///
/// Parses `sentinel = b".."`, and `sentinel(value = b"..", len(..))` entry by entry. A bare
/// `sentinel`, a list without `value`, and an unknown or duplicate entry are errors
impl TryFrom<&syn::Meta> for ParsedSentinel {
    type Error = syn::Error;
    fn try_from(input: &syn::Meta) -> syn::Result<Self> {
        match input {
            // --------------------------------------------------
            // `sentinel = b".."`
            // --------------------------------------------------
            syn::Meta::NameValue(nv) => Ok(Sentinel {
                value: tk_syn_macros::helpers::parse_nv(nv)?,
                len: None,
            }),
            // --------------------------------------------------
            // `sentinel(value = b"..", len(..))`
            // --------------------------------------------------
            syn::Meta::List(list) => {
                let mut value: Option<syn::Lit> = None;
                let mut len: Option<(syn::Path, Box<HeaderXcoder>)> = None;
                list.parse_nested_meta(|meta| {
                    tk_syn_macros::handle_unique_nested_meta_values! {
                        meta;
                        err!(UnknownSentinelField(meta.path));
                        value: symbol::parse_pnm_sentinel_value => err!(DuplicateSentinelValue),
                        len: HeaderXcoder::parse_pnm_len        => err!(DuplicateSentinelLen),
                    }
                })?;
                let Some(value) = value else {
                    return Err(syn::Error::new_spanned(
                        &list.path,
                        err!(MissingSentinelValue),
                    ));
                };
                Ok(Sentinel { value, len })
            }
            // --------------------------------------------------
            // a bare `sentinel`
            // --------------------------------------------------
            syn::Meta::Path(path) => Err(syn::Error::new_spanned(path, err!(SentinelWithoutValue))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        ast::{attr::MainContainer, symbol},
    };

    use quote::quote;

    /// Verifies every malformed `sentinel` is the one error reported, naming it
    ///
    /// Each case writes one malformed `sentinel` beside an otherwise complete key + len framing: a
    /// bare path, a list without `value`, an unknown entry, a second `value`, a sentinel `len(..)`
    /// of `size(exact = 0)`, and one without the `dec` the container needs
    #[test]
    fn a_malformed_sentinel_is_the_one_error() {
        // --------------------------------------------------
        // (name, sentinel, the one error)
        // --------------------------------------------------
        let cases = [
            ("bare", quote! { sentinel }, Error::SentinelWithoutValue),
            (
                "no value",
                quote! { sentinel(len(dec = decb::be_u16_as_usize)) },
                Error::MissingSentinelValue,
            ),
            (
                "unknown entry",
                quote! { sentinel(value = b"S", size(exact = 2)) },
                Error::UnknownSentinelField(symbol::SIZE.to_string()),
            ),
            (
                "duplicate value",
                quote! { sentinel(value = b"S", value = b"T") },
                Error::DuplicateSentinelValue,
            ),
            (
                "zero-width len",
                quote! { sentinel(value = b"S", len(dec = decb::u8_as_usize, size(exact = 0))) },
                Error::LenZeroWidth,
            ),
            (
                "len without dec",
                quote! { sentinel(value = b"S", len(enc = encb::u8_from_usize)) },
                Error::MissingDecInCodec(symbol::LENGTH.to_string()),
            ),
        ];
        for (name, sentinel, expected) in cases {
            let item: syn::DeriveInput = syn::parse_quote! {
                #[klv(
                    #sentinel,
                    key(dec = decb::u8),
                    len(dec = decb::u8_as_usize),
                    allow_unimplemented_encode,
                )]
                struct Reading {
                    #[klv(key = 0x01, dec = decb::u8)]
                    id: u8,
                }
            };
            let Err(messages) = MainContainer::parsed(&item) else {
                panic!("{name}: reports an error");
            };
            assert_eq!(
                messages,
                [expected.to_string()],
                "{name}: the one error names the sentinel"
            );
        }
    }
}
