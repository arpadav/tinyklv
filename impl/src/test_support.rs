//! The setup the unit tests share: a derive input parsed into its container, as the derive
//! parses it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{Ctxt, ast::attr::MainContainer};

/// [`MainContainer`] implementation of the unit tests' setup
impl<'a> MainContainer<'a> {
    /// Parses `item` through a fresh [`Ctxt`], as the derive does
    ///
    /// # Arguments
    ///
    /// * `item` - The derive input
    ///
    /// # Returns
    ///
    /// The container
    ///
    /// # Errors
    ///
    /// The message of each error the input raises, in the order raised
    pub(crate) fn parsed(item: &'a syn::DeriveInput) -> Result<Self, Vec<String>> {
        let cx = Ctxt::new();
        let container = Self::from_ast(&cx, item);
        match cx.check() {
            // `from_ast` returns `None` only beside an error
            Ok(()) => container.ok_or_else(Vec::new),
            Err(errors) => Err(errors.into_iter().map(|error| error.to_string()).collect()),
        }
    }
}
