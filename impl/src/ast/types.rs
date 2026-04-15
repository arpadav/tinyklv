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
        if let Ok(_) = fork.parse::<syn::Path>() {
            if fork.peek(syn::Token![!]) {
                let mcro: syn::Macro = input.parse()?;
                return Ok(XcoderLike::Macro(mcro));
            }
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
