//! [`ErrorKind`], what a decode error is, for a caller to match on instead of the error's text
//!
//! A decode error is winnow's [`ContextError`], which has no slot for a kind. The kind rides as a
//! `StrContext::Label` tag after the error's own label: the error's display names only its first
//! label, so a tag changes neither the display nor the cause chain
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::{ContextError, StrContext};

/// What a decode error is, for a caller to match on instead of the error's text
///
/// [`ErrorKind::of`] reads the kind of a [`crate::Result`] error, and
/// [`DecodeIterError::kind`](crate::decoder::DecodeIterError::kind) that of a [`crate::Decoder`]
/// error, and [`PartialError::kind`](crate::traits::PartialError::kind) that of a streaming
/// decode's. Every error of the BER decoders carries one, as do the UTF-16 decoders' odd-length
/// rejection and every error the derive's decodes and sentinel seek return. An error built
/// elsewhere, by a bare `winnow` parser such as a fixed-width integer's at the end of the input,
/// or by a custom decoder, has none
///
/// # Example
///
/// ```rust
/// use tinyklv::{ErrorKind, dec::ber::ber_length};
///
/// // a long-form BER length claiming two length bytes, with one present
/// let error = ber_length(&mut &[0x82, 0x01][..]).unwrap_err();
/// assert_eq!(ErrorKind::of(&error), Some(ErrorKind::Truncated));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The input ends before the key, length or value being read does, such as a long-form BER
    /// length short of its length bytes
    Truncated,

    /// A value's bytes are not a valid encoding of its type, such as a BER value past its target
    /// integer type
    MalformedValue,

    /// An item's length is not one its value can have, such as an odd byte count of UTF-16
    WrongLength,

    /// An item consumed no input, so the decode loop would read it again forever
    NoProgress,

    /// An item's key is none of its container's, under `deny_unknown_keys`
    UnknownKey,

    /// A required field has no value once the input is decoded
    MissingRequired,

    /// A `break_on` returned [`BreakType::Abort`](crate::BreakType::Abort)
    Aborted,

    /// No sentinel begins a frame in the input
    MissingSentinel,
}
/// [`ErrorKind`] implementation
impl ErrorKind {
    /// Every kind, which [`ErrorKind::of`] reads a tag back against
    const ALL: [Self; 8] = [
        Self::Truncated,
        Self::MalformedValue,
        Self::WrongLength,
        Self::NoProgress,
        Self::UnknownKey,
        Self::MissingRequired,
        Self::Aborted,
        Self::MissingSentinel,
    ];

    /// The kind `error` was last tagged with
    ///
    /// A site that classifies a failure again tags it after the site before it, so the
    /// outermost classification wins
    ///
    /// # Arguments
    ///
    /// * `error` - a decode error
    ///
    /// # Returns
    ///
    /// The kind, or `None` when nothing tagged `error`
    #[must_use]
    pub fn of(error: &ContextError) -> Option<Self> {
        error
            .context()
            .filter_map(|context| match context {
                StrContext::Label(label) => Self::ALL.into_iter().find(|kind| kind.tag() == *label),
                _ => None,
            })
            .last()
    }

    /// The `StrContext::Label` text tagging an error with this kind, named after the kind's path
    /// so that no label describing a value reads the same
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::Truncated => "tinyklv::ErrorKind::Truncated",
            Self::MalformedValue => "tinyklv::ErrorKind::MalformedValue",
            Self::WrongLength => "tinyklv::ErrorKind::WrongLength",
            Self::NoProgress => "tinyklv::ErrorKind::NoProgress",
            Self::UnknownKey => "tinyklv::ErrorKind::UnknownKey",
            Self::MissingRequired => "tinyklv::ErrorKind::MissingRequired",
            Self::Aborted => "tinyklv::ErrorKind::Aborted",
            Self::MissingSentinel => "tinyklv::ErrorKind::MissingSentinel",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorKind;
    use crate::__export::{labeled_error, with_kind};
    use winnow::stream::Stream as _;

    #[test]
    /// Tests that an untagged error has no kind, and that a tag reads back and leaves the
    /// error's display as its label's
    fn a_tag_reads_back_and_leaves_the_display() {
        let input: &[u8] = &[];
        let labeled = labeled_error(&input, &input.checkpoint(), "value");
        let display = labeled.to_string();
        assert_eq!(ErrorKind::of(&labeled), None);
        let tagged = with_kind(labeled, ErrorKind::WrongLength);
        assert_eq!(ErrorKind::of(&tagged), Some(ErrorKind::WrongLength));
        assert_eq!(tagged.to_string(), display);
    }

    #[test]
    /// Tests that the last tag wins, so an outer site's kind overrides an inner site's
    fn the_last_tag_wins() {
        let input: &[u8] = &[];
        let inner = with_kind(
            labeled_error(&input, &input.checkpoint(), "value"),
            ErrorKind::Truncated,
        );
        let outer = with_kind(inner, ErrorKind::MalformedValue);
        assert_eq!(ErrorKind::of(&outer), Some(ErrorKind::MalformedValue));
    }
}
