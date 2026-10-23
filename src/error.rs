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
/// decode's. Every error of the BER and string decoders carries one, as do the `as_date!`
/// family's over a string decoder and every error the derive's decodes and sentinel seek return.
/// An error built elsewhere, by a bare `winnow` parser such as a fixed-width integer's at the end
/// of the input, or by a custom decoder, has none, unless [`ErrorKind::tag`] tags it
///
/// # Example
///
/// ```rust
/// use tinyklv::{ErrorKind, dec::string::to_string_utf16_le};
///
/// // three bytes of UTF-16, whose code units are two bytes each
/// let error = to_string_utf16_le(3)(&mut &[0x41, 0x00, 0x42][..]).unwrap_err();
/// assert_eq!(ErrorKind::of(&error), Some(ErrorKind::WrongLength));
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
                StrContext::Label(label) => {
                    Self::ALL.into_iter().find(|kind| kind.label() == *label)
                }
                _ => None,
            })
            .last()
    }

    /// Tags `error` with this kind, for [`ErrorKind::of`] to read back
    ///
    /// A custom decoder tags the errors it can classify. A packet length tagged
    /// [`ErrorKind::MalformedValue`] can never be valid, so a [`crate::Decoder`] reports its
    /// packet once and moves on, where it waits for more bytes after any other length failure
    ///
    /// The tag rides as a `StrContext::Label` after `error`'s own, and the display of a
    /// [`ContextError`] names only its first label, so label `error` before tagging it: the tag
    /// of an unlabelled error is the label its display names
    ///
    /// # Arguments
    ///
    /// * `error` - the error to tag
    ///
    /// # Returns
    ///
    /// `error`, tagged with this kind
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::{decoder::DecodeIterError, prelude::*};
    /// use tinyklv::{dec::binary as decb, enc::binary as encb};
    /// use tinyklv::winnow::error::{ContextError, StrContext};
    ///
    /// /// A packet length past 64 is never valid
    /// fn packet_len(input: &mut &[u8]) -> tinyklv::Result<usize> {
    ///     let len = decb::u8_as_usize(input)?;
    ///     if len > 64 {
    ///         let mut error = ContextError::new();
    ///         error.push(StrContext::Label("packet length past 64"));
    ///         return Err(ErrorKind::MalformedValue.tag(error));
    ///     }
    ///     Ok(len)
    /// }
    ///
    /// #[derive(Klv, Debug, PartialEq)]
    /// #[klv(
    ///     sentinel = b"\xAA",
    ///     key(dec = decb::u8, enc = encb::u8),
    ///     len(dec = packet_len, enc = encb::u8_from_usize),
    /// )]
    /// struct Ping {
    ///     #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    ///     id: u8,
    /// }
    ///
    /// // a packet of length 200, then one of length 3 holding `id` 7
    /// let mut decoder = Ping::decoder();
    /// decoder.feed(&[0xAA, 200, 0xAA, 0x03, 0x01, 0x01, 0x07]);
    /// let result = decoder.next_fresh();
    /// assert_eq!(
    ///     result.as_ref().err().and_then(DecodeIterError::kind),
    ///     Some(ErrorKind::MalformedValue),
    /// );
    /// assert_eq!(decoder.next(), Some(Ping { id: 7 }));
    /// ```
    #[must_use]
    #[cold]
    #[inline]
    pub fn tag(self, mut error: ContextError) -> ContextError {
        error.push(StrContext::Label(self.label()));
        error
    }

    /// The `StrContext::Label` text tagging an error with this kind, named after the kind's path
    /// so that no label describing a value reads the same
    pub(crate) const fn label(self) -> &'static str {
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
