//! Error accumulation context for proc-macro attribute parsing
//!
//! Provides [`Ctxt`], a collection type that gathers [`syn::Error`] values
//! during attribute parsing and combines them into a single diagnostic at the
//! end of a parse pass. Dropping a [`Ctxt`] without calling [`Ctxt::check`]
//! panics, enforcing that all accumulated errors are explicitly handled
//!
//! Taken from: [<https://github.com/serde-rs/serde/blob/930401b0dd58a809fce34da091b8aa3d6083cb33/serde_derive/src/internals/ctxt.rs>]
//!
//! Author: aav
#![allow(clippy::unwrap_used, clippy::panic)]
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::cell::RefCell;
use std::fmt::Display;

#[derive(Default)]
/// A type to collect errors together and format them
///
/// Errors are pushed via [`Ctxt::error_spanned_by`] or [`Ctxt::syn_error`]
/// during attribute parsing. When parsing is complete, call [`Ctxt::check`]
/// to consume the context and return a combined [`syn::Error`] if any errors
/// were recorded, or `Ok(())` otherwise
///
/// Dropping this object without calling [`Ctxt::check`] will cause a panic,
/// enforcing that all errors are surfaced rather than silently discarded
/// References can be shared since this type uses run-time exclusive mut
/// checking via [`RefCell`]
pub(crate) struct Ctxt {
    /// Error accumulator - set to `None` once [`Ctxt::check`] is called to
    /// prevent double-checking
    errors: RefCell<Option<Vec<syn::Error>>>,
}

/// [`Ctxt`] implementation
impl Ctxt {
    /// Creates a new, empty [`Ctxt`]
    ///
    /// The returned context contains no errors but will still trigger a panic
    /// on drop if [`Ctxt::check`] is never called
    pub fn new() -> Self {
        Ctxt {
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    /// Adds a spanned error to the context using a tokenizable object for span
    /// information
    ///
    /// `obj` is converted to a [`proc_macro2::TokenStream`] to extract source
    /// span data. `msg` is the human-readable diagnostic message. Curbs
    /// monomorphization by delegating to `syn::Error::new_spanned` with the
    /// erased token stream rather than keeping a generic call site per type
    ///
    /// # Arguments
    ///
    /// * `obj` - Any tokenizable value used to anchor the error's source span
    /// * `msg` - The diagnostic message to attach to the error
    pub fn error_spanned_by<A: ToTokens, T: Display>(&self, obj: A, msg: T) {
        // --------------------------------------------------
        // convert the tokenizable object to a stream for
        // span extraction, then push the spanned error
        // --------------------------------------------------
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            .push(syn::Error::new_spanned(obj.into_token_stream(), msg));
    }

    /// Adds a pre-constructed [`syn::Error`] to the context
    ///
    /// Use this when the error already carries correct span information from
    /// a `syn` parse operation and no additional spanning is needed
    ///
    /// # Arguments
    ///
    /// * `err` - The [`syn::Error`] to record
    pub fn syn_error(&self, err: syn::Error) {
        self.errors.borrow_mut().as_mut().unwrap().push(err);
    }

    /// Consumes this context and returns a combined error if any were recorded
    ///
    /// Takes the accumulated error vector out of the [`RefCell`], combines all
    /// errors into a single [`syn::Error`] using [`syn::Error::combine`], and
    /// returns it as `Err`. Returns `Ok(())` if no errors were recorded. Sets
    /// the inner `Option` to `None` to signal that checking has occurred,
    /// preventing the drop-panic from triggering
    pub fn check(self) -> syn::Result<()> {
        // --------------------------------------------------
        // take the error vector out of the refcell, leaving
        // `None` so the drop impl does not panic
        // --------------------------------------------------
        let mut errors = self.errors.borrow_mut().take().unwrap().into_iter();
        // --------------------------------------------------
        // return `Ok` immediately if no errors were recorded
        // --------------------------------------------------
        let Some(mut combined) = errors.next() else {
            return Ok(());
        };
        // --------------------------------------------------
        // combine all remaining errors into the first one
        // --------------------------------------------------
        for rest in errors {
            combined.combine(rest);
        }
        Err(combined)
    }
}

/// [`Ctxt`] implementation of [`Drop`]
impl Drop for Ctxt {
    fn drop(&mut self) {
        // --------------------------------------------------
        // panic if the context was dropped without checking,
        // unless a panic is already in progress
        // --------------------------------------------------
        assert!(
            std::thread::panicking() || self.errors.borrow().is_none(),
            "forgot to check for errors"
        );
    }
}
