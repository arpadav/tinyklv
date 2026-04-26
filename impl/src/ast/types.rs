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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Dispatch-intent sigil for encoders
///
/// Written before the path/macro in `#[klv(enc = <sigil><fn>)]`:
///
/// * `None`  (`enc = func`)  -> emit `func(&self.field)` - fn takes `&T`
///   (deref coercion handles `&String -> &str`, `&Vec<u8> -> &[u8]`, etc.)
/// * `Ref`   (`enc = &func`) -> emit `func(EncodeAs::encode_as(&self.field))` -
///   dispatches via the [`EncodeAs`](`tinyklv::traits::EncodeAs`) trait:
///   primitives pass by value (Copy), `String -> &str`, `Vec<T> -> &[T]`,
///   `Box<T>/Rc<T>/Arc<T> -> &T`. No clone, no heap allocation
/// * `Deref` (`enc = *func`) -> emit `func(self.field)` - fn takes `T` by value
///   (for `Copy` types and small primitives)
pub(crate) enum XcoderSigil {
    None,
    Ref,
    Deref,
}

#[derive(Debug, Clone)]
/// Wraps an [`XcoderLike`] with an optional leading dispatch sigil
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
            let _: syn::Token![*] = input.parse()?;
            XcoderSigil::Deref
        } else {
            XcoderSigil::None
        };
        let inner: XcoderLike = input.parse()?;
        Ok(SiguledXcoder { sigil, inner })
    }
}
/// [`SiguledXcoder`] implementation of [`ToTokens`]
///
/// This does not need the sigil, since that describes the fn signature
/// but does not affect how the emitted code is structured
impl ToTokens for SiguledXcoder {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.inner {
            XcoderLike::Path(path) => path.to_tokens(tokens),
            XcoderLike::Macro(mcro) => mcro.to_tokens(tokens),
            XcoderLike::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Clone)]
/// Post-decode conversion/mutation function for the `latebind` field attribute
///
/// Written after `latebind =` in `#[klv(latebind = <mut?><fn>)]`:
///
/// * `latebind = path`       (`is_mut == false`) -> emit
///   `.map(path)` after the decoder's `.ok()`; `path` has signature
///   `Fn(T) -> U` where `T` is the decoder output and `U` is the field type
/// * `latebind = &mut path`  (`is_mut == true`)  -> emit
///   `.map(|mut __v| { path(&mut __v); __v })`; `path` has signature
///   `Fn(&mut T)` with `T == U`
///
/// The plain `&` (without `mut`) form is rejected at parse time - the
/// consuming form has no `&` variant.
pub(crate) struct LatebindXcoder {
    pub(crate) is_mut: bool,
    pub(crate) inner: XcoderLike,
}
#[derive(Debug, Clone)]
/// Field-level `default` attribute value
///
/// * [`DefaultValue::Call`] - bare `default`; emits
///   `<T as ::core::default::Default>::default()` for the field's type
/// * [`DefaultValue::Expr`] - `default = <expr>`; emits the inline expression
pub(crate) enum DefaultValue {
    Call,
    Expr(syn::Expr),
}

/// [`LatebindXcoder`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for LatebindXcoder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let is_mut = if input.peek(syn::Token![&]) && input.peek2(syn::Token![mut]) {
            let _: syn::Token![&] = input.parse()?;
            let _: syn::Token![mut] = input.parse()?;
            true
        } else if input.peek(syn::Token![&]) {
            return Err(input.error(
                "`&` without `mut` on `latebind` is invalid: use `latebind = path` \
                 (consuming, Fn(T) -> U) or `latebind = &mut path` (mutating, Fn(&mut T))",
            ));
        } else {
            false
        };
        let inner: XcoderLike = input.parse()?;
        Ok(LatebindXcoder { is_mut, inner })
    }
}
