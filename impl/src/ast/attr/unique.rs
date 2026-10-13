//! The value of a setting that may be written once
//!
//! Defines [`Unique`], what the values written for one setting come to: none, one, or more than
//! one, the duplicate reported. A field's settings are merged this way across its `#[klv(..)]`
//! lists, and a type's `default(..)` settings across its entries; one list's parser rejects a
//! setting it repeats itself
//!
//! Author: aav

/// The values written for one setting, of which at most one is allowed
#[derive(Debug)]
pub(super) enum Unique<T> {
    /// No value is written
    Unset,
    /// One value is written
    One(T),
    /// More than one value is written, and the second is reported; the first of them
    Duplicate(T),
}
/// [`Unique`] implementation
impl<T> Unique<T> {
    /// Takes the values written for a setting, reporting the second one, if any
    ///
    /// # Arguments
    ///
    /// * `values` - The values, in the order written; none after the second is taken
    /// * `duplicate` - Reports the second value, which it is passed
    ///
    /// # Returns
    ///
    /// The values' [`Unique`]
    pub(super) fn of(values: impl IntoIterator<Item = T>, duplicate: impl FnOnce(&T)) -> Self {
        let mut values = values.into_iter();
        let Some(first) = values.next() else {
            return Self::Unset;
        };
        match values.next() {
            Some(second) => {
                duplicate(&second);
                Self::Duplicate(first)
            }
            None => Self::One(first),
        }
    }

    /// The first value written
    ///
    /// # Returns
    ///
    /// The value, a duplicate's first included, or `None` when none is written
    pub(super) fn first(self) -> Option<T> {
        match self {
            Self::Unset => None,
            Self::One(value) | Self::Duplicate(value) => Some(value),
        }
    }

    /// The value written once
    ///
    /// # Returns
    ///
    /// The value, or `None` when none or more than one is written
    pub(super) fn once(self) -> Option<T> {
        match self {
            Self::One(value) => Some(value),
            Self::Unset | Self::Duplicate(_) => None,
        }
    }
}
