//! Shared helpers for the codec unit tests
//!
//! [`ContextErrorExt`] pins which `Expected` context a BER error carries
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::{ContextError, StrContext, StrContextValue};

/// `Expected`-context queries on a decoder's error
pub(crate) trait ContextErrorExt {
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
}

impl ContextErrorExt for ContextError {
    fn expects(&self, description: &str) -> bool {
        self.context().any(|context| {
            matches!(
                context,
                StrContext::Expected(StrContextValue::Description(expected)) if *expected == description
            )
        })
    }
}
