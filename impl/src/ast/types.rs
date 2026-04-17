// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// outward-facing types, can change
// --------------------------------------------------
pub(crate) type TypeType = syn::Type;
pub(crate) type XcoderType = XcoderLike;

#[derive(Debug, Clone)]
/// encoder / decoders can be:
///
/// * a path
/// * a macro call
/// * a fn call which returns impl FnOnce
pub(crate) enum XcoderLike {
    Path(syn::Path),
    Expr(syn::Expr),
    Macro(syn::Macro),
}

/// Dispatch-intent sigil for field encoders
///
/// Written before the path/macro in `#[klv(enc = <sigil><fn>)]`:
///
/// * `None` (`enc = func`)  → emit `func(&self.field)` - fn takes `&T`
///   (deref coercion handles `&String → &str`, `&Vec<u8> → &[u8]`, etc.)
/// * `Ref`  (`enc = &func`) → emit `func(EncodeAs::encode_as(&self.field))` -
///   dispatches via the [`EncodeAs`](tinyklv::traits::EncodeAs) trait:
///   primitives pass by value (Copy), `String → &str`, `Vec<T> → &[T]`,
///   `Box<T>/Rc<T>/Arc<T> → &T`. No clone, no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XcoderSigil {
    None,
    Ref,
}

/// Wraps an [`XcoderLike`] with an optional leading dispatch sigil
///
/// Only used for field-level encoders. Container-level `key.enc` / `len.enc` /
/// `default.enc` keep raw [`XcoderLike`] because their call shape has no
/// owned/borrowed ambiguity.
#[derive(Debug, Clone)]
pub(crate) struct SiguledXcoder {
    pub(crate) sigil: XcoderSigil,
    pub(crate) inner: XcoderLike,
}
/// [`SiguledXcoder`] implementation of [`syn::parse::Parse`]
///
/// Consumes an optional leading `&` or `*` before delegating to
/// [`XcoderLike::parse`]. No change to [`XcoderLike::parse`] itself.
impl syn::parse::Parse for SiguledXcoder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let sigil = if input.peek(syn::Token![&]) {
            let _: syn::Token![&] = input.parse()?;
            XcoderSigil::Ref
        } else if input.peek(syn::Token![*]) {
            return Err(input.error(
                "`*` sigil removed: use `&func` - EncodeAs dispatch already covers \
                 String→&str, Vec→&[T], Box/Rc/Arc→&T, primitives-by-value",
            ));
        } else {
            XcoderSigil::None
        };
        let inner: XcoderLike = input.parse()?;
        Ok(SiguledXcoder { sigil, inner })
    }
}
/// [`XcoderLike`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for XcoderLike {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        // --------------------------------------------------
        // prioritize macro: check for a path followed by '!'
        // --------------------------------------------------
        let fork = input.fork();
        if fork.parse::<syn::Path>().is_ok() && fork.peek(syn::Token![!]) {
            let mcro: syn::Macro = input.parse()?;
            return Ok(XcoderLike::Macro(mcro));
        }
        drop(fork);
        // --------------------------------------------------
        // expressions
        // --------------------------------------------------
        if input.peek(syn::token::Paren)
            || input.peek(syn::token::Brace)
            || input.peek(syn::Token![if])
            || input.peek(syn::Token![match])
        {
            if let Ok(x) = input.parse::<syn::Expr>() {
                return Ok(XcoderLike::Expr(x));
            }
        }
        // --------------------------------------------------
        // check paths last
        // --------------------------------------------------
        if let Ok(x) = input.parse::<syn::Path>() {
            return Ok(XcoderLike::Path(x));
        }
        // --------------------------------------------------
        // otherwise error
        // --------------------------------------------------
        Err(lookahead.error())
    }
}
/// [`XcoderLike`] implementation of [`ToTokens`]
impl ToTokens for XcoderLike {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            XcoderLike::Path(path) => path.to_tokens(tokens),
            XcoderLike::Macro(mcro) => mcro.to_tokens(tokens),
            XcoderLike::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}
