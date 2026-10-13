//! [`ContextErrorExt`], the queries a test runs on a decoder's [`ContextError`]
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use std::{error::Error, iter};
use winnow::error::{ContextError, StrContext, StrContextValue};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// How every label tagging an error with its `ErrorKind` begins, the kind's path
const KIND_TAG: &str = "tinyklv::ErrorKind::";

/// Queries on a decoder's error: its context, its display and its cause chain
pub(crate) trait ContextErrorExt {
    /// The labels of the error's context chain
    ///
    /// # Returns
    ///
    /// Every `StrContext::Label` of the error, field first: a derived error keeps only the first
    /// label the value decoder added, and lists it after the label naming the field. The label
    /// that tags the error with its `ErrorKind` is not one
    fn labels(&self) -> impl Iterator<Item = &'static str>;

    /// Whether the error's context chain carries `Expected(description)`
    ///
    /// # Arguments
    ///
    /// * `description` - the expected-value description to look for
    fn expects(&self, description: &str) -> bool;

    /// Whether a BER overflow error's context chain expects a value that fits the target type
    fn expects_fits_target(&self) -> bool {
        self.expects("a value that fits the target integer type")
    }

    /// Whether a BER length error's context chain expects the bytes its long form claims
    fn expects_length_bytes(&self) -> bool {
        self.expects("enough bytes in stream for length encoding")
    }

    /// Whether the error's display contains every needle, each after the one before it
    ///
    /// # Arguments
    ///
    /// * `needles` - the substrings, in the order the display must name them
    fn names_in_order(&self, needles: &[&str]) -> bool;

    /// The error's cause chain
    ///
    /// # Returns
    ///
    /// The error's cause, then each error down that cause's `source` chain; empty if the error
    /// has no cause
    fn causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)>;

    /// The outermost error of type `E` in the error's cause chain
    ///
    /// # Returns
    ///
    /// The first error of [`ContextErrorExt::causes`] that is an `E`; none if no error of the
    /// chain is
    fn find_cause<E: Error + 'static>(&self) -> Option<&E> {
        self.causes().find_map(|cause| cause.downcast_ref())
    }
}

/// [`ContextError`] implementation of [`ContextErrorExt`]
impl ContextErrorExt for ContextError {
    fn labels(&self) -> impl Iterator<Item = &'static str> {
        self.context().filter_map(|context| match context {
            StrContext::Label(label) if !label.starts_with(KIND_TAG) => Some(*label),
            _ => None,
        })
    }

    fn expects(&self, description: &str) -> bool {
        self.context().any(|context| {
            matches!(
                context,
                StrContext::Expected(StrContextValue::Description(expected)) if *expected == description
            )
        })
    }

    fn names_in_order(&self, needles: &[&str]) -> bool {
        let display = self.to_string();
        needles
            .iter()
            .try_fold(display.as_str(), |rest, needle| {
                rest.split_once(needle).map(|(_, after)| after)
            })
            .is_some()
    }

    fn causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)> {
        let cause = self.cause().map(|cause| cause as &(dyn Error + 'static));
        // the deref copies the chain's reference out, so `source` borrows for the chain's lifetime
        iter::successors(cause, |cause| (*cause).source())
    }
}
