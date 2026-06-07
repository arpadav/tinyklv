//! Parsed model for the container-level `#[klv(break_on = ..)]` attribute
//!
//! `break_on` accepts two forms, distinguished by the right-hand side of the name-value:
//!
//! * a **literal** (e.g. `break_on = 0xFF`) - codegen lowers it to "stop the loop when the decoded
//!   key equals this literal" ([`crate::BreakType::Done`] on match, else `Proceed`)
//! * a **function path** (e.g. `break_on = classify`) - codegen calls `path(key, len)`, which the
//!   user writes as `fn(key, len) -> tinyklv::BreakType`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// The two forms a `break_on = ..` right-hand side may take
///
/// Consumed by the decode codegen to build the per-iteration break decision: a literal lowers to a
/// key-equality check yielding [`crate::BreakType::Done`]; a function path is called directly with
/// the concrete `(key, len)`
pub(crate) enum BreakOnSpec {
    /// `break_on = <literal>` - a decoded key equal to this literal stops the loop
    Literal(syn::Lit),

    /// `break_on = <path>` - a user function `fn(key, len) -> BreakType` consulted per triple
    Func(syn::Path),
}

/// [`BreakOnSpec`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for BreakOnSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakOnSpec::Literal(lit) => write!(f, "Literal({})", lit.to_token_stream()),
            BreakOnSpec::Func(path) => write!(f, "Func({})", path.to_token_stream()),
        }
    }
}

/// Newtype wrapper paralleling [`super::sentinel::Sentinel`]: `Some` once a `break_on = ..` has been
/// parsed, `None` when the name-value was for some other keyword
pub(crate) struct BreakOn(pub Option<BreakOnSpec>);

/// [`BreakOn`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for BreakOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// [`BreakOn`] implementation of [`TryFrom`] for a [`syn::MetaNameValue`]
///
/// Returns `Ok(BreakOn(None))` when the name-value is not the `break_on` keyword (so the caller can
/// fall through), `Ok(BreakOn(Some(..)))` for a literal or path value, and `Err` when the value is
/// neither (e.g. `break_on = "oops"` written as a non-literal expression like an array)
impl TryFrom<&syn::MetaNameValue> for BreakOn {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaNameValue) -> syn::Result<Self> {
        // --------------------------------------------------
        // not the `break_on` keyword: nothing to parse
        // --------------------------------------------------
        if symbol::Symbol::from(&input.path) != symbol::BREAK_ON {
            return Ok(BreakOn(None));
        }
        // --------------------------------------------------
        // distinguish the literal form from the function-path form by the rhs expression
        // --------------------------------------------------
        match &input.value {
            syn::Expr::Lit(expr_lit) => {
                Ok(BreakOn(Some(BreakOnSpec::Literal(expr_lit.lit.clone()))))
            }
            syn::Expr::Path(expr_path) => {
                Ok(BreakOn(Some(BreakOnSpec::Func(expr_path.path.clone()))))
            }
            other => Err(syn::Error::new_spanned(other, err!(InvalidBreakOn))),
        }
    }
}
